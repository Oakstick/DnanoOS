"""The fifth stage's keyed clips (`block_foe.py` runs them): the tülükuş (all of it: an owl), and the
albastı's smother over her Mixamo clips (`mixamo_foe.py` first, then `block_foe.py -- albasti RIG OUT
--onto albasti_m.blend`). Each folklore move shows what the code's pose shows (`view/duel.rs`).
"""
import math

from block_foe import RALLY, TAU, E, Pose, bake, env, move_marks

TALONS = (30, 6, 34)
SMOTHER = (26, 6, 34)
# The talons' rise: frames to the top of it and the frame the dive begins (`duelist::ALOFT`).
ALOFT = (10, 22)


class Tulukus:
    """The owl stands and stalks on its talons, its wings folded, its head turning on its neck; for its
    talons it rises straight up on spread wings over him (the code lifts it) and drops talons first."""

    def wings(self, p, spread, fold=0.0, beat=0.0):
        """Its wings `spread` degrees out from its sides, the primaries folded `fold` back in, beating
        `beat` degrees."""
        p.turn("wing.L", E(0, 0, spread + beat))
        p.turn("wing.R", E(0, 0, -spread - beat))
        p.turn("hand.L", E(0, 0, -fold))
        p.turn("hand.R", E(0, 0, fold))
        return p

    def base(self, f, n=160):
        p = Pose()
        # (Its head swivels as an owl's does, a long way, and holds.)
        turn = 55 * math.sin(TAU * f / n) ** 3
        p.turn("head", E(2 * math.sin(TAU * f / 47), turn, 0))
        p.turn("body", E(1.2 * math.sin(TAU * f / 60), 0, 0))
        return self.wings(p, 0.0, 0.0, 1.5 * max(0.0, math.sin(TAU * f / 90)))

    def idle(self, f):
        return self.base(f)

    def walk(self, f):
        """It stalks on its talons, rolling from foot to foot, its head held still."""
        n = 40
        p = self.base(0)
        for s, ph in (("L", 0.0), ("R", 0.5)):
            u = (f / n + ph) % 1.0
            swing = -18 + 36 * (u / 0.5) if u < 0.5 else 18 - 36 * ((u - 0.5) / 0.5)
            lift = 0.0 if u < 0.5 else 25 * math.sin(math.pi * (u - 0.5) / 0.5)
            p.turn(f"leg.{s}", E(swing - lift * 0.5, 0, 0))
            p.turn(f"foot.{s}", E(lift, 0, 0))
        p.turn("body", E(4, 0, 7 * math.sin(TAU * f / n)))
        p.turn("head", E(0, 0, -7 * math.sin(TAU * f / n)))
        p["loc"] = (0.0, 0.01 * abs(math.cos(TAU * f / n)), 0.0)
        return p

    def talons(self, f):
        """Up on spread wings, beating hard, its legs drawn up under it; over him it tips forward and
        drops, talons first, the wings swept up behind; then it settles and folds them."""
        s, a, r = TALONS
        n = s + a + r
        up, down = ALOFT
        spread = env(f, [(0, 0.0), (5, 1.0), (s + a + 6, 1.0), (n, 0.0)])
        dive = env(f, [(down - 2, 0.0), (s, 1.0), (s + a + 2, 1.0), (s + a + 14, 0.0)])
        beat = 30 * math.sin(TAU * f / 9) * (1 - dive) * spread
        p = self.base(f)
        self.wings(p, 75 * spread + 25 * dive, -20 * spread, beat)
        p.turn("wing.L", E(-25 * dive, 0, 0))
        p.turn("wing.R", E(-25 * dive, 0, 0))
        p.turn("body", E(-10 * spread * (1 - dive) + 40 * dive, 0, 0))
        p.turn("head", E(-30 * dive, 0, 0))
        for side in ("L", "R"):
            p.turn(f"leg.{side}", E(-50 * spread * (1 - dive) - 60 * dive, 0, 0))
            p.turn(f"foot.{side}", E(40 * spread * (1 - dive) - 20 * dive, 0, 0))
        return p

    def rally(self, f):
        """Its omen: wings thrown open, head up, crying."""
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.base(f)
        self.wings(p, 80 * k, -15 * k, 8 * math.sin(TAU * f / 6) * k)
        p.turn("head", E(-25 * k, 0, 0))
        p.turn("body", E(-8 * k, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.base(f)
        self.wings(p, 35 * k, 10 * k)
        p.turn("body", E(-15 * k, 0, 8 * k))
        p.turn("head", E(-20 * k, 25 * k, 0))
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        sway = math.sin(TAU * f / 20)
        p = self.base(f)
        self.wings(p, (30 + 20 * sway) * k, 15 * k)
        p.turn("body", E(10 * k, 0, 14 * sway * k))
        p.turn("head", E(20 * k, 30 * sway * k, 0))
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.base(f)
        self.wings(p, 60 * k, 0)
        p.turn("body", E(-25 * k, 0, 0))
        p["loc"] = (0.0, 0.0, -0.12 * k)
        return p

    def make(self, arm):
        bake(arm, "idle", 160, self.idle, loop=True)
        bake(arm, "walk", 40, self.walk, loop=True, stride=0.5)
        bake(arm, "talons", sum(TALONS), self.talons, marks=move_marks(TALONS))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


class Albasti:
    """The smother: she rears up tall beside him, on her toes, her long arms spread over him; then she
    falls forward on him, her arms closing round; then she draws back."""

    def smother(self, f):
        s, a, r = SMOTHER
        rise = env(f, [(0, 0.0), (s, 1.0), (s + 2, 0.7), (s + a + 6, 0.0)])
        fall = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 6, 1.0), (s + a + r, 0.0)])
        p = Pose()
        p["loc"] = (0.0, 0.08 * rise - 0.06 * fall, 0.25 * fall)
        p.turn("spine", E(-12 * rise + 25 * fall, 0, 0))
        p.turn("chest", E(-8 * rise + 15 * fall, 0, 0))
        p.turn("head", E(-15 * rise + 10 * fall, 0, 0))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{side}", E(-150 * rise - 70 * fall, 0, sg * (35 * rise - 15 * fall)))
            p.turn(f"forearm.{side}", E(-20 * rise - 40 * fall, 0, 0))
            p.turn(f"thigh.{side}", E(-10 * fall, 0, 0))
            p.turn(f"shin.{side}", E(20 * fall, 0, 0))
            p.turn(f"foot.{side}", E(25 * rise, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = Pose()
        p.turn("chest", E(-15 * k, 0, 0))
        p.turn("head", E(-25 * k, 0, 0))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{side}", E(-90 * k, 0, sg * 50 * k))
        return p

    def make(self, arm):
        bake(arm, "smother", sum(SMOTHER), self.smother, marks=move_marks(SMOTHER))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))


FOES = {"tulukus": Tulukus(), "albasti": Albasti()}
