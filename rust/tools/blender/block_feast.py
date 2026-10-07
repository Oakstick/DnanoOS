"""Keyed clips for the courts' and the lair's own that Mixamo has nothing for (`block_foe.py` bakes them,
the princes' onto their Mixamo .blend: `block_foe.py -- FOE RIG OUT --onto FOE_m.blend`), and the keyed
bones laid under their Mixamo clips (`mixamo_foe.py`, `lay`). Each shows what the code's pose shows.

- `Charge`: a prince's charge (Amon's fiery charge, Asmodeus's bull charge, Krampus's hoof charge): down
  low, head down and a foot pawing the floor through the wind-up; the run itself is his run clip
  (`view::foe_clips`); then up out of the lean as he pulls up.
- `BellyRoll`: Beelzebub rocked back and tucked, over and over on his belly (the code rolls his whole
  body), then up and dizzy.
- `BeelzebubWings`: his wings never still, buzzing faster as he walks.
- `Swarm`: a fly swarm, keyed whole: its heart bobbing, its four clouds turning about it two one way
  and two the other; its bite the clouds drawn in and the heart thrown at him.
"""
import math

from block_courts import DEG, phases
from block_foe import BITE, RALLY, TAU, E, Pose, bake, env, move_marks, smooth

CHARGES = {"fiery_charge": (40, 40, 44), "bull_charge": (40, 46, 46), "hoof_charge": (42, 46, 46)}
BELLY_ROLL = (40, 46, 46)


def crouch(p, k, lean=0.0, head=0.0):
    """Down low `k` of the way, bent forward `lean` degrees more, the head `head` degrees down."""
    p.turn("spine", E(14 * k + lean * 0.6, 0, 0))
    p.turn("chest", E(8 * k + lean * 0.4, 0, 0))
    p.turn("head", E(head, 0, 0))
    for s in ("L", "R"):
        p.turn(f"thigh.{s}", E(-30 * k, 0, 0))
        p.turn(f"shin.{s}", E(45 * k, 0, 0))
        p.turn(f"foot.{s}", E(-15 * k, 0, 0))
        p.turn(f"upper_arm.{s}", E(-12 * k, 0, 0))
        p.turn(f"forearm.{s}", E(-15 * k, 0, 0))
    return p


class Charge:
    """A prince's charge, keyed whole (his weapon arm as sculpted), baked as `name`."""

    def __init__(self, name):
        self.name = name
        self.fd = CHARGES[name]

    def pose(self, f):
        s, a, r = self.fd
        if f < s:
            k = smooth(f / s)
            p = crouch(Pose(), k, 12 * k, 25 * k)
            # (A foot pawing the floor as he lowers his head.)
            paw = abs(math.sin(TAU * f / 18)) * k
            p.turn("thigh.R", E(-20 * paw, 0, 0))
            p.turn("shin.R", E(25 * paw, 0, 0))
            return p
        if f < s + a:
            return crouch(Pose(), 0.6, 18, 20)
        u = 1 - smooth((f - s - a) / r)
        return crouch(Pose(), 0.6 * u, 18 * u, 20 * u)

    def make(self, arm):
        bake(arm, self.name, sum(self.fd), self.pose, marks=move_marks(self.fd))


class BellyRoll:
    """Beelzebub's belly roll, keyed whole (his whole body's roll is the code's)."""

    def pose(self, f):
        s, a, r = BELLY_ROLL
        p = Pose()
        if f < s:
            k = smooth(f / s)
            p.turn("spine", E(-15 * k, 0, 0))
            tuck, head = 46 * k, 0.0
        elif f < s + a:
            k = 1.0
            p.turn("spine", E(30, 0, 0))
            p.turn("chest", E(15, 0, 0))
            for side in ("L", "R"):
                p.turn(f"thigh.{side}", E(-60, 0, 0))
                p.turn(f"shin.{side}", E(80, 0, 0))
            tuck, head = 92.0, 46.0
        else:
            u = 1 - smooth((f - s - a) / r)
            sway = 7 * math.sin(TAU * f / 54) * u
            p.turn("spine", E(0, 0, sway))
            p.turn("chest", E(0, 0, sway))
            tuck, head = 20 * u, 17 * u
        for side in ("L", "R"):
            p.turn(f"upper_arm.{side}", E(-tuck, 0, 0))
            p.turn(f"forearm.{side}", E(-tuck * 0.8, 0, 0))
        p.turn("head", E(head, 0, 0))
        return p

    def make(self, arm):
        bake(arm, "belly_roll", sum(BELLY_ROLL), self.pose, marks=move_marks(BELLY_ROLL))


class BeelzebubWings:
    """His wings (`wing.L`, `wing.R`) buzzing, as the code flutters them: turned (fl/2, -0.1 + 0.3 fl,
    0.12 + fl) radians, mirrored on the right."""

    def wings(self, f, buzz, amp):
        fl = math.sin(f / 60.0 * buzz) * amp
        p = Pose()
        p.turn("wing.L", E(fl * 0.5 * DEG, (-0.1 + fl * 0.3) * DEG, (0.12 + fl) * DEG))
        p.turn("wing.R", E(fl * 0.5 * DEG, (0.1 - fl * 0.3) * DEG, (-0.12 - fl) * DEG))
        return p

    def idle(self, f):
        return self.wings(f, 30.0, 0.08)

    def walk(self, f):
        return self.wings(f, 46.0, 0.16)


CLOUDS = ["cloud.1", "cloud.2", "cloud.3", "cloud.4"]
# Each cloud's turns in the swarm's loop (its six seconds), two one way and two the other.
TURNS = [1, -2, 2, -1]


class Swarm:
    """A fly swarm: its heart bobbing in the air, its clouds turning about it."""

    def clouds(self, p, f, n=360, spin=1.0, tilt=0.12):
        t = f / 60.0
        for k, (b, turns) in enumerate(zip(CLOUDS, TURNS)):
            p.turn(b, E(math.sin(t * 1.1 + k) * tilt * DEG, 360.0 * turns * spin * f / n, math.cos(t * 0.9 + k * 2) * tilt * DEG))
        return p

    def idle(self, f):
        p = self.clouds(Pose(), f)
        p["loc"] = (0.0, 0.08 * math.sin(TAU * 2 * f / 360), 0.0)
        p.turn("heart", E(0, 0, 3 * math.sin(TAU * 3 * f / 360)))
        return p

    def fly(self, f):
        p = self.clouds(Pose(), f, 180)
        p["loc"] = (0.0, 0.08 * math.sin(TAU * f / 90), 0.0)
        p.turn("heart", E(9, 0, 3 * math.sin(TAU * f / 90)))
        return p

    def bite(self, f):
        s, a, r = BITE
        draw = env(f, [(0, 0.0), (s, 1.0), (s + 2, 0.0)])
        out = env(f, [(s - 1, 0.0), (s + 2, 1.0), (s + a + 2, 1.0), (s + a + r, 0.0)])
        p = self.clouds(Pose(), f, 120)
        p["loc"] = (0.0, 0.0, -0.12 * draw + 0.35 * out)
        k = 1 - 0.25 * draw + 0.1 * out
        p["scale"] = {b: (k, k, k) for b in CLOUDS}
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.clouds(Pose(), f, 120)
        g = 1 + 0.25 * k
        p["scale"] = {b: (g, g, g) for b in CLOUDS}
        p["loc"] = (0.0, 0.15 * k, 0.0)
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.clouds(Pose(), f)
        p["loc"] = (0.0, 0.0, -0.15 * k)
        g = 1 + 0.2 * k
        p["scale"] = {b: (g, g, g) for b in CLOUDS}
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        p = self.clouds(Pose(), f, 360, 1 + 2 * k, 0.12 + 0.2 * k)
        g = 1 + 0.3 * k
        p["scale"] = {b: (g, g, g) for b in CLOUDS}
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.clouds(Pose(), f)
        p["loc"] = (0.0, 0.0, -0.2 * k)
        return p

    def make(self, arm):
        bake(arm, "hover", 360, self.idle, loop=True)
        bake(arm, "fly", 180, self.fly, loop=True, stride=1.4)
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


FOES = {
    "fly_swarm": Swarm(),
    "beelzebub": BellyRoll(),
    "amon": Charge("fiery_charge"),
    "asmodeus": Charge("bull_charge"),
    "krampus": Charge("hoof_charge"),
}
