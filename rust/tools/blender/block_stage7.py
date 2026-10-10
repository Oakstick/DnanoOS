"""The seventh stage's keyed clips (`block_foe.py` runs them): Níðhöggr. Its burrow shows what the code's
pose shows (`view/duel.rs`): down into the floor head first (the code sinks it and hides it while it is
under), and up out of it beneath him with its neck flung up; its bite a strike off its coils.
"""
import math

from block_foe import BITE, RALLY, TAU, E, Pose, bake, env, move_marks

BURROW = (60, 6, 40)
# The frame the code has it under the floor (`duelist::BURROW_SINK`).
SINK = 10
NECKS = ["neck.1", "neck.2", "neck.3", "neck.4", "neck.5"]


class Nidhogg:
    def sway(self, p, f, period, amp, lag=0.1):
        total = 0.0
        for i, b in enumerate(NECKS):
            a = amp * math.sin(TAU * (f / period - i * lag))
            total += a
            p.turn(b, E(0.5 * amp * math.sin(TAU * (f / period - i * lag + 0.25)), 0, a))
        p.turn("head", E(0, 6 * math.sin(TAU * f / (period * 2)), -total * 0.5))
        p.turn("tail", E(0, 14 * math.sin(TAU * f / period), 0))
        return p

    def idle(self, f):
        return self.sway(Pose(), f, 120, 2.5)

    def walk(self, f):
        n = 60
        p = self.sway(Pose(), f, n, 4.5, 0.13)
        p.turn("coil", E(0, 6 * math.sin(TAU * (f / n - 0.2)), 2 * math.sin(TAU * f / n)))
        p["loc"] = (0.05 * math.sin(TAU * f / n), 0.0, 0.0)
        return p

    def bite(self, f):
        s, a, r = BITE
        back = env(f, [(0, 0.0), (s, 1.0), (s + 1, 0.4), (s + 3, 0.0)])
        out = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 2, 1.0), (s + a + r, 0.0)])
        p = self.sway(Pose(), f, 120, 2.5 * (1 - max(back, out)))
        for i, b in enumerate(NECKS):
            p.turn(b, E(-5 * back * (1 + i * 0.2) + 10 * out * (1.2 - i * 0.12), 0, 0))
        p.turn("head", E(12 * back - 15 * out, 0, 0))
        p["loc"] = (0.0, 0.0, 0.2 * out)
        return p

    def burrow(self, f):
        """Down into the floor head first; then, under it, nothing shows; up out of it beneath him, the
        neck flung up and the jaws on him."""
        s, a, r = BURROW
        n = s + a + r
        dive = env(f, [(0, 0.0), (SINK, 1.0), (s - 4, 1.0), (s, 0.0)])
        rise = env(f, [(s - 2, 0.0), (s + 3, 1.0), (s + a + 6, 0.8), (n, 0.0)])
        p = self.sway(Pose(), f, 120, 1.0)
        for i, b in enumerate(NECKS):
            p.turn(b, E(14 * dive - 8 * rise * (1 - i * 0.1), 0, 0))
        p.turn("head", E(25 * dive - 10 * rise, 0, 0))
        p.turn("coil", E(10 * dive, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.sway(Pose(), f, 30, 4.0 * k)
        for b in NECKS:
            p.turn(b, E(-4 * k, 0, 0))
        p.turn("head", E(-20 * k, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.sway(Pose(), f, 120, 2.5)
        for b in NECKS:
            p.turn(b, E(-4 * k, 0, 3 * k))
        p.turn("head", E(-14 * k, 15 * k, 0))
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        p = self.sway(Pose(), f, 20, 2.5 + 6 * k, 0.16)
        p.turn("head", E(14 * k, 0, 0))
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.sway(Pose(), f, 120, 2.5)
        for b in NECKS:
            p.turn(b, E(-6 * k, 0, 0))
        p.turn("head", E(-16 * k, 0, 0))
        return p

    def make(self, arm):
        bake(arm, "idle", 120, self.idle, loop=True)
        bake(arm, "slither", 60, self.walk, loop=True, stride=1.2)
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "burrow", sum(BURROW), self.burrow, marks=move_marks(BURROW))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


FOES = {"nidhogg": Nidhogg()}
