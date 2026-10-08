"""The story's cast where Mixamo has nothing (ANIMATION.md §12), keyed by script: Quitmer, the old
man's dog (asleep on his straw, rising, straining and snapping at the end of his chain, backing down
and lying again, watching, his tail going); the hug of Scene II (the freed boy's arms round the
kneeling hero's neck); and Scene I's Krampus's grab (the reach through the burst gate, the claws
closing on the boy, the lift and the bellow) and his arm held up with his catch, laid over his run.

    blender -b -P tools/blender/block_cast.py -- WHO WHO_rig.glb OUT.blend [--onto IN.blend]

WHO is `dog`, `kid` or `krampus_shade`; the rig is what `--export-rig dog` (`kid0`, `krampus_shade`)
wrote. With `--onto`, the keyed clips are added to a .blend that has the Mixamo ones already (the
boys'). Every clip is a function of the frame, baked at 60 fps (`block_foe.bake`), then
`export_clips.py` as for any clip. Angles are degrees about the game's axes (x pitches: a hanging
limb swings back, the body and the head tip forward; y turns toward its left; z tips: a limb out to
its left rises), relative to the bone's parent; the root's place in metres.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import vh_blender as vh  # noqa: E402
from block_foe import E, Pose, Wolf, bake, env, smooth  # noqa: E402

TAU = 2 * math.pi


# ------------------------------------------------------------------ Quitmer


class Dog(Wolf):
    """A mastiff the size of a pony: heavy, slow to rise, his head big and low."""

    def __init__(self):
        super().__init__(trot=40, gallop=30, strides=(0.7, 1.6), heavy=1.4)

    def lying(self, f, head_up=1.0):
        """Down on his straw like a sphinx: the body lowered onto its belly, the forelegs folded
        flat before him, the hind legs tucked under; the head up and watching (`head_up` 1) or laid
        on his paws asleep (0)."""
        p = Pose()
        breath = math.sin(TAU * f / (150 if head_up < 0.5 else 110))
        p["loc"] = (0.0, -0.17 + 0.004 * breath, -0.02)
        p.turn("hips", E(-4, 0, 0))
        p.turn("chest", E(5 + 1.5 * breath, 0, 0))
        for side in ("L", "R"):
            # (The elbow on the floor behind the shoulder, the forearm and paw flat before him.)
            self.leg(p, "f", side, 55, -140, 85)
            # (The hock under the hip, the shank forward along the floor.)
            self.leg(p, "b", side, -100, 175, -75)
        p.turn("neck", E(30 - 38 * head_up + 2 * breath, 0, 0))
        p.turn("head", E(28 - 30 * head_up, 0, 0))
        p.turn("jaw", E(1.5 * max(0.0, breath) * head_up, 0, 0))
        p.turn("tail.1", E(70, 25, 0))
        p.turn("tail.2", E(30, 30, 0))
        p.turn("tail.3", E(20, 25, 0))
        return p

    def sleep(self, f):
        p = self.lying(f, 0.0)
        # (An ear's twitch, a dream.)
        p.turn("head", E(0, 0, 3 * max(0.0, math.sin(TAU * f / 180)) ** 6))
        return p

    def lie(self, f):
        """Lying, head up, watching the boys; his head turning a little after them."""
        p = self.lying(f, 1.0)
        p.turn("neck", E(0, 10 * math.sin(TAU * f / 240), 0))
        p.turn("head", E(4 * math.sin(TAU * f / 120), 4 * math.sin(TAU * (f / 240 - 0.2)), 0))
        return p

    def lie_wag(self, f):
        """Lying, calmed, his tail thumping the straw."""
        p = self.lie(f)
        w = math.sin(TAU * f / 20)
        p.turn("tail.1", E(-30, 30 * w, 0))
        p.turn("tail.2", E(0, 25 * w, 0))
        p.turn("tail.3", E(0, 20 * w, 0))
        return p

    def idle(self, f):
        p = super().idle(f)
        p.turn("tail.1", E(15, 0, 0))
        return p

    def stand_up(self, f, n=50):
        """From lying to his feet: the forelegs first, then the hind end heaving up."""
        a = self.lying(f, 1.0)
        b = self.idle(f)
        u = smooth(f / n)
        front = smooth(min(1.0, f / (n * 0.6)))
        p = a.mix(b, u)
        for side in ("L", "R"):
            fa, fb = a.mix(b, front), b
            for bone in (f"upper_f.{side}", f"lower_f.{side}", f"paw_f.{side}"):
                p[bone] = fa.get(bone, fb.get(bone))
        return p

    def lie_down(self, f, n=70):
        """Back down onto the straw: the hind end first, then the forelegs sliding out."""
        a = self.idle(f)
        b = self.lying(f, 1.0)
        u = smooth(f / n)
        back = smooth(min(1.0, f / (n * 0.6)))
        p = a.mix(b, u)
        for side in ("L", "R"):
            ba = a.mix(b, back)
            for bone in (f"upper_b.{side}", f"lower_b.{side}", f"paw_b.{side}"):
                p[bone] = ba.get(bone, p.get(bone))
        return p

    def snarl(self, f, n=68):
        """At the end of his chain: braced, forelegs planted wide before him, hind legs driving,
        hackles and head low and thrust at them; a lunge each cycle (the chain stops it), the jaws
        snapping twice, the lips back."""
        lunge = max(0.0, math.sin(TAU * f / n)) ** 2
        snap = abs(math.sin(TAU * 2 * f / n)) ** 0.7
        p = Pose()
        p["loc"] = (0.0, -0.05 + 0.03 * lunge, 0.04 + 0.08 * lunge)
        p.turn("hips", E(4 - 6 * lunge, 0, 2 * math.sin(TAU * f / n)))
        p.turn("chest", E(-6 + 10 * lunge, 0, 0))
        p.turn("neck", E(14 - 22 * lunge, 4 * math.sin(TAU * f / n * 3), 0))
        p.turn("head", E(-12 - 10 * lunge, 0, 0))
        p.turn("jaw", E(6 + 34 * snap, 0, 0))
        for side, s in (("L", 1), ("R", -1)):
            self.leg(p, "f", side, -22 - 18 * lunge, 12 + 6 * lunge, -10)
            p.turn(f"upper_f.{side}", E(0, 0, 6 * s))
            self.leg(p, "b", side, 18 + 10 * lunge, -18, 12)
            p.turn(f"upper_b.{side}", E(0, 0, 4 * s))
        p.turn("tail.1", E(-50, 0, 0))
        p.turn("tail.2", E(-10, 6 * math.sin(TAU * f / 8), 0))
        return p

    def make(self, arm):
        bake(arm, "idle", 120, self.idle, loop=True)
        bake(arm, "walk", self.trot, self.trot_pose, loop=True, stride=self.strides[0])
        bake(arm, "sleep", 180, self.sleep, loop=True)
        bake(arm, "lie", 240, self.lie, loop=True)
        bake(arm, "lie_wag", 240, self.lie_wag, loop=True)
        bake(arm, "stand_up", 50, self.stand_up, marks={"recover_end": 50})
        bake(arm, "lie_down", 70, self.lie_down, marks={"recover_end": 70})
        bake(arm, "snarl", 68, self.snarl, loop=True)


# ------------------------------------------------------------------ the boys' hug


class Kid:
    def hug(self, f, n=120):
        """Up on his toes, his arms thrown round a neck before him a little above his own head,
        his cheek laid on a shoulder (his head turned to his left); a sob now and then shaking his
        shoulders."""
        sob = max(0.0, math.sin(TAU * f / 40)) ** 3 * env(f % n, [(0, 1.0), (n * 0.5, 0.4), (n, 1.0)])
        p = Pose()
        p["loc"] = (0.0, 0.012 + 0.003 * sob, 0.01)
        p.turn("spine", E(6, 0, 0))
        p.turn("chest", E(4 - 3 * sob, 0, 0))
        p.turn("neck", E(-8, 18, 0))
        p.turn("head", E(-6, 22, -10))
        for side, s in (("L", 1), ("R", -1)):
            # (The upper arm raised forward past level and in toward the middle; the forearm bent in
            # round the neck, the hand on the back of it.)
            p.turn(f"upper_arm.{side}", E(-118 + 4 * sob, 0, -s * 22))
            p.turn(f"forearm.{side}", E(-10, -s * 70, 0))
            p.turn(f"hand.{side}", E(0, -s * 20, 0))
        for side in ("L", "R"):
            # (On his toes.)
            p.turn(f"thigh.{side}", E(-4, 0, 0))
            p.turn(f"shin.{side}", E(6, 0, 0))
            p.turn(f"foot.{side}", E(-22, 0, 0))
        return p

    @staticmethod
    def arms_thrown_up(p, f, k=1.0, wide=30.0, shake=4.0):
        """Both arms flung up over his head and out (`wide` degrees), the hands open, trembling."""
        for side, s in (("L", 1), ("R", -1)):
            tr = shake * math.sin(TAU * f / 7 + (0.0 if side == "L" else 1.7))
            p.turn(f"upper_arm.{side}", E(0, 0, s * wide * k))
            p.turn(f"upper_arm.{side}", E((-172 + tr) * k, 0, 0))
            p.turn(f"forearm.{side}", E(-8 * k, 0, 0))
            p.turn(f"hand.{side}", E(-15 * k, 0, s * 10 * k))
        return p

    def arms_up(self, f):
        """The arms alone (laid over a yell): flung up and wide, shaking."""
        return self.arms_thrown_up(Pose(), f, 1.0, 11.0, 6.0)

    def alarm(self, f):
        """Alarm: recoiling, his weight thrown back onto a bent back leg, his arms flung up before his
        face, palms out, his head back and turned a little away; trembling all over."""
        tr = math.sin(TAU * f / 6)
        p = Pose()
        p["loc"] = (0.0, -0.025 + 0.002 * tr, -0.04)
        p.turn("spine", E(-10, 0, 0))
        p.turn("chest", E(-8 + tr, 6, 0))
        p.turn("neck", E(-10, -8, 0))
        p.turn("head", E(-14, -10, 0))
        for side, s in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{side}", E(0, 0, s * 18))
            p.turn(f"upper_arm.{side}", E(-145 + 4 * tr, 0, 0))
            p.turn(f"forearm.{side}", E(-55, -s * 12, 0))
            p.turn(f"hand.{side}", E(-35, 0, 0))
        # (One foot back, both knees bent under the recoil.)
        p.turn("thigh.L", E(-22, 0, 4))
        p.turn("shin.L", E(28, 0, 0))
        p.turn("foot.L", E(-6, 0, 0))
        p.turn("thigh.R", E(14, 0, -4))
        p.turn("shin.R", E(30, 0, 0))
        p.turn("foot.R", E(-16, 0, 0))
        return p

    def reach_out(self, f):
        """After him: his right arm flung out at the gate, his left up over his head (laid over a yell)."""
        tr = math.sin(TAU * f / 8)
        p = Pose()
        p.turn("upper_arm.R", E(0, 0, -8))
        p.turn("upper_arm.R", E(-95 + 3 * tr, 0, 0))
        p.turn("forearm.R", E(-8, 0, 0))
        p.turn("hand.R", E(-10, 0, 0))
        p.turn("upper_arm.L", E(0, 0, 35))
        p.turn("upper_arm.L", E(-168 + 4 * tr, 0, 0))
        p.turn("forearm.L", E(-12, 0, 0))
        return p

    def make(self, arm):
        bake(arm, "hug", 120, self.hug, loop=True)
        bake(arm, "arms_up", 42, self.arms_up, loop=True)
        bake(arm, "alarm", 48, self.alarm, loop=True)
        bake(arm, "reach_out", 48, self.reach_out, loop=True)


# ------------------------------------------------------------------ Krampus in Scene I


# The grab as the story stages it: from the gate bursting (frame 0) to his turn back into the fog
# (`logic::story::stage`: GATE_BURST 1.3, GRAB 1.5, TURN 2.05 s).
GRAB_AT = 12
GRAB_N = 45


class KrampusShade:
    def grab(self, f):
        """Through the burst gate: lunging in low, his right claw thrust out before him; on `grab`
        it closes, and he heaves it up high with his catch, rearing back on his goat's legs to
        bellow, his left arm flung out."""
        reach = env(f, [(0, 0.6), (GRAB_AT - 2, 1.0), (GRAB_AT + 6, 0.2), (GRAB_N, 0.0)])
        lift = env(f, [(GRAB_AT, 0.0), (GRAB_AT + 14, 1.0), (GRAB_N, 1.0)])
        rear = env(f, [(GRAB_AT, 0.0), (GRAB_AT + 12, 1.0), (GRAB_N - 6, 0.8), (GRAB_N, 0.5)])
        close = env(f, [(GRAB_AT - 3, 0.0), (GRAB_AT + 1, 1.0)])
        p = Pose()
        p["loc"] = (0.0, -0.05 * reach + 0.02 * rear, 0.12 * reach - 0.06 * rear)
        p.turn("hips", E(10 * reach - 8 * rear, 0, 0))
        p.turn("spine", E(12 * reach - 14 * rear, 0, 0))
        p.turn("chest", E(10 * reach - 16 * rear, -10 * reach + 8 * lift, 0))
        p.turn("neck", E(-6 * reach - 10 * rear, 0, 0))
        p.turn("head", E(-14 * reach - 22 * rear + 3 * math.sin(TAU * f / 7) * rear, 0, 0))
        # The right arm (his own right: `.R`): out before him, then up with the boy.
        p.turn("upper_arm.R", E(-75 * reach - 130 * lift, 0, -18 * lift))
        p.turn("forearm.R", E(-20 * reach + 25 * lift, 0, 0))
        p.turn("hand.R", E(35 * close - 15 * lift, 0, 0))
        # The left flung out wide as he rears.
        p.turn("upper_arm.L", E(-30 * reach + 10 * rear, 0, 55 * rear))
        p.turn("forearm.L", E(-35 * rear, 0, 0))
        for side, s in (("L", 1), ("R", -1)):
            p.turn(f"thigh.{side}", E(-20 * reach * (1 if side == "R" else 0.3) + 6 * rear, 0, 0))
            p.turn(f"shin.{side}", E(14 * reach, 0, 0))
        return p

    def hold_up(self, f, n=40):
        """His right arm raised with his catch, shaking it as he lopes (laid over his run's arm)."""
        shake = math.sin(TAU * f / n)
        p = Pose()
        p.turn("upper_arm.R", E(-150 + 6 * shake, 0, -20))
        p.turn("forearm.R", E(25 + 8 * shake, 0, 0))
        p.turn("hand.R", E(10, 0, 0))
        return p

    def make(self, arm):
        bake(arm, "grab", GRAB_N, self.grab, marks={"grab": GRAB_AT, "recover_end": GRAB_N})
        bake(arm, "hold_up", 40, self.hold_up, loop=True)


CAST = {"dog": Dog(), "kid": Kid(), "krampus_shade": KrampusShade()}


def main():
    a = vh.args()
    if len(a) < 3 or a[0] not in CAST:
        sys.exit(__doc__)
    who, rig, out = a[0], a[1], a[2]
    vh.fresh_scene()
    arm = vh.import_rig(rig)
    if len(a) > 4 and a[3] == "--onto":
        bpy.ops.wm.open_mainfile(filepath=os.path.abspath(a[4]))
        arm = vh.armature()
    if arm.animation_data is None:
        arm.animation_data_create()
    CAST[who].make(arm)
    bpy.context.scene.render.fps = vh.FPS
    first = next((n for n in ("idle", "sleep", "grab", "hug") if n in bpy.data.actions), None)
    if first:
        arm.animation_data.action = bpy.data.actions[first]
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out))
    print("block_cast: saved", out)


if __name__ == "__main__":
    main()
