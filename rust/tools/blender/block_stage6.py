"""The sixth stage's keyed clips (`block_foe.py` runs them): the seven-headed ejder (all of it), and the
gashadokuro's hanging jaw, laid under its Mixamo clips by `mixamo_foe.py` (`lay`). Each folklore move
shows what the code's pose shows (`view/duel.rs`).
"""
import math

from block_foe import RALLY, TAU, E, Pose, bake, env, move_marks

HEADS = (30, 24, 34)
GRAB = (40, 8, 44)
SLAM = (42, 6, 48)


class GashaJaw:
    """The gashadokuro's jaw, hanging open from its hinges: rattling as it stands and walks, gaping as
    its hands reach to seize (and rattling as it holds him), dropping with its slam, wide for its cry."""

    def idle(self, f):
        p = Pose()
        p.turn("jaw", E(8 + 5 * abs(math.sin(TAU * f / 17)), 0, 0))
        return p

    def grab(self, f):
        s, a, r = GRAB
        gape = env(f, [(0, 0.0), (s, 1.0), (s + a, 1.0), (s + a + r, 0.0)])
        rattle = 8 * abs(math.sin(TAU * f / 6)) * env(f, [(s + a, 0.0), (s + a + 4, 1.0), (s + a + r - 8, 1.0), (s + a + r, 0.0)])
        p = Pose()
        p.turn("jaw", E(8 + 30 * gape + rattle, 0, 0))
        return p

    def slam(self, f):
        s, a, r = SLAM
        k = env(f, [(0, 0.0), (s, 1.0), (s + a + 4, 0.6), (s + a + r, 0.0)])
        p = Pose()
        p.turn("jaw", E(8 + 22 * k, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        k = env(f, [(0, 0.0), (s, 1.0), (s + 20, 1.0), (s + a + r, 0.0)])
        p = Pose()
        p.turn("jaw", E(8 + 35 * k + 4 * math.sin(TAU * f / 5) * k, 0, 0))
        return p


class Ejder:
    """Seven necks weaving out of time with one another; the legs plod; the tail sweeps."""

    def weave(self, p, f, period=110, amp=1.0):
        for k in range(7):
            ph = f / period + k * 0.13
            p.turn(f"neck{k}.a", E(3 * amp * math.sin(TAU * ph), 0, 3 * amp * math.sin(TAU * (ph * 0.5 + 0.2))))
            p.turn(f"neck{k}.b", E(4 * amp * math.sin(TAU * (ph - 0.1)), 0, 4 * amp * math.sin(TAU * (ph * 0.5 + 0.1))))
            p.turn(f"head{k}", E(-4 * amp * math.sin(TAU * (ph - 0.2)), 8 * amp * math.sin(TAU * (ph * 0.5 + k * 0.3)), 0))
        p.turn("tail", E(0, 12 * math.sin(TAU * f / (period * 2)), 0))
        return p

    def idle(self, f):
        p = self.weave(Pose(), f)
        p["loc"] = (0.0, 0.01 * math.sin(TAU * f / 110), 0.0)
        return p

    def walk(self, f):
        n = 60
        p = self.weave(Pose(), f, 30, 1.3)
        for name, ph in (("leg_f.L", 0.0), ("leg_b.R", 0.0), ("leg_f.R", 0.5), ("leg_b.L", 0.5)):
            u = (f / n + ph) % 1.0
            swing = -16 + 32 * (u / 0.6) if u < 0.6 else 16 - 32 * ((u - 0.6) / 0.4)
            lift = 0.0 if u < 0.6 else 25 * math.sin(math.pi * (u - 0.6) / 0.4)
            p.turn(name, E(swing - lift * 0.3, 0, 0))
            p.turn(name + ".low", E(lift, 0, 0))
        p.turn("body", E(0, 0, 3 * math.sin(TAU * f / n)))
        p["loc"] = (0.0, 0.02 * math.cos(TAU * 2 * f / n), 0.0)
        p.turn("tail", E(0, 18 * math.sin(TAU * f / n), 0))
        return p

    def heads(self, f):
        """Every head drawn back, then they strike in turn, two by two, the middle last."""
        s, a, r = HEADS
        n = s + a + r
        wind = env(f, [(0, 0.0), (s, 1.0), (s + a, 1.0), (n, 0.0)])
        per = a / 4.0
        p = self.weave(Pose(), f, 110, 0.4)
        p["loc"] = (0.0, -0.03 * wind, 0.0)
        p.turn("body", E(6 * wind, 0, 0))
        for b, necks in enumerate([(2, 4), (1, 5), (0, 6), (3,)]):
            start = s + b * per
            k = 0.0 if f < start else min(2.0, (f - start) / per)
            out = k if k <= 1.0 else 2.0 - k
            out = max(out, 0.0) * (1.0 - env(f, [(s + a, 0.0), (n, 1.0)]))
            for k7 in necks:
                p.turn(f"neck{k7}.a", E(-10 * wind * (1 - out) + 18 * out, 0, 0))
                p.turn(f"neck{k7}.b", E(-12 * wind * (1 - out) + 24 * out, 0, 0))
                p.turn(f"head{k7}", E(8 * wind * (1 - out) + 10 * out, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.weave(Pose(), f, 40, 1.0 + k)
        for k7 in range(7):
            p.turn(f"neck{k7}.a", E(-12 * k, 0, 0))
            p.turn(f"head{k7}", E(-25 * k, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.weave(Pose(), f)
        for k7 in range(7):
            p.turn(f"neck{k7}.b", E(-10 * k, 0, 6 * k * (1 if k7 % 2 else -1)))
        p.turn("body", E(-5 * k, 0, 0))
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        p = self.weave(Pose(), f, 20, 1 + 3 * k)
        for k7 in range(7):
            p.turn(f"neck{k7}.a", E(14 * k, 0, 0))
        p.turn("body", E(0, 0, 6 * math.sin(TAU * f / 24) * k))
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.weave(Pose(), f)
        for k7 in range(7):
            p.turn(f"neck{k7}.a", E(-14 * k, 0, 0))
        return p

    def make(self, arm):
        bake(arm, "idle", 110 * 2, self.idle, loop=True)
        bake(arm, "walk", 60, self.walk, loop=True, stride=1.4)
        bake(arm, "heads", sum(HEADS), self.heads, marks=move_marks(HEADS))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


FOES = {"ejder": Ejder()}
