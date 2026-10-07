"""Envy's court keyed by script (`block_foe.py` bakes them): Leviathan and his serpentlings. Every move
shows what the code's pose shows (`view/duel.rs`: `levi_neck`, his neck bent, his head turned more,
his jaw opened), his body's twist, lean and swell keyed on his root.
"""
import math

from block_courts import DEG, phases
from block_foe import BITE, RALLY, TAU, E, Pose, bake, env, move_marks

ENVY = {
    "coil_lash": (34, 8, 44), "brine_spit": (34, 1, 34), "dive": (60, 6, 56), "covet": (44, 6, 40), "covet_fire": (44, 1, 36),
    "covet_thunder": (44, 6, 40), "covet_beam": (44, 8, 40), "covet_wave": (44, 6, 46), "bell_toll": (72, 6, 46),
    "burning_coil": (46, 6, 40), "roar": (40, 1, 36),
}
# The frame the code has him under the water (`duelist::BURROW_SINK`).
SINK = 10
NECKS = ["neck.1", "neck.2", "neck.3"]
# How a bend of his neck is shared down its three bones (the code bends it whole at its root).
SHARE = (0.5, 0.3, 0.2)


class Leviathan:
    """The serpent on his coil: his neck weaving, his head riding it, his jaw working, his fins
    fanning, his tail lashing; moving, he slides from side to side."""

    def sway(self, p, t, moving=False):
        z = math.sin(t) * (0.08 if moving else 0.05)
        x = math.sin(t * 0.5) * 0.04
        for b, k in zip(NECKS, SHARE):
            p.turn(b, E(x * k * DEG, 0, z * k * DEG))
        p.turn("head", E(math.sin(t * 0.7) * 0.05 * DEG, math.sin(t * 0.4) * 0.06 * DEG, -math.sin(t) * 0.04 * DEG))
        p.turn("jaw", E((0.05 + abs(math.sin(t * 1.9)) * 0.05) * DEG, 0, 0))
        for b, side in (("fin.L", 1.0), ("fin.R", -1.0)):
            p.turn(b, E(0, 0, side * (0.12 + math.sin(t * 2.4) * 0.14) * DEG))
        tail = math.sin(t * 2.0) * 0.25
        p.turn("tail.1", E(0, tail * 0.6 * DEG, 0))
        p.turn("tail.2", E(0, tail * DEG, 0))
        return p

    def neck(self, p, neck, head, jaw):
        for b, k in zip(NECKS, SHARE):
            p.turn(b, E(neck * k * DEG, 0, 0))
        p.turn("head", E(head * DEG, 0, 0))
        p.turn("jaw", E(jaw * DEG, 0, 0))
        return p

    def t(self, f):
        return f / 60.0 * 0.9

    def idle(self, f):
        return self.sway(Pose(), self.t(f))

    def slither(self, f):
        t = TAU * f / 90
        p = self.sway(Pose(), t, moving=True)
        p.turn("body", E(0, math.sin(t) * 0.08 * DEG, 0))
        return p

    def spit(self, f, fd, deep=0.35, head=0.25):
        wind, strike, rec, snap = phases(f, fd)
        out = 1 - rec
        neck = -deep * wind if strike == 0 else (-deep + 2 * deep * snap) * out
        return self.neck(self.sway(Pose(), self.t(f)), neck, -head * wind * out, (0.25 * wind + 0.45 * snap) * out)

    def brine_spit(self, f):
        return self.spit(f, ENVY["brine_spit"])

    def covet(self, f):
        return self.spit(f, ENVY["covet"])

    def covet_fire(self, f):
        return self.spit(f, ENVY["covet_fire"])

    def covet_beam(self, f):
        return self.spit(f, ENVY["covet_beam"])

    def covet_thunder(self, f):
        wind, strike, rec, snap = phases(f, ENVY["covet_thunder"])
        out = 1 - rec
        neck = -0.5 * wind if strike == 0 else (-0.5 + 0.8 * snap) * out
        return self.neck(self.sway(Pose(), self.t(f)), neck, -0.5 * wind * out, 0.5 * wind * out)

    def covet_wave(self, f):
        """He rears up tall, then throws himself down on the floor."""
        wind, strike, rec, snap = phases(f, ENVY["covet_wave"])
        out = 1 - rec
        p = self.sway(Pose(), self.t(f))
        if strike == 0:
            p.turn("body", E(-0.15 * wind * DEG, 0, 0))
            p["scale"] = {"body": (1 - 0.04 * wind, 1 + 0.1 * wind, 1)}
        else:
            p.turn("body", E((-0.15 + 0.45 * snap) * out * DEG, 0, 0))
            p["scale"] = {"body": (1 + 0.05 * snap * out, 1 - 0.08 * snap * out, 1)}
        return self.neck(p, 0.2 * snap * out, 0.0, 0.4 * snap * out)

    def coil_lash(self, f):
        """Wound round to one side, his tail drawn back; then the lash right across."""
        wind, strike, rec, snap = phases(f, ENVY["coil_lash"])
        out = 1 - rec
        yaw = -0.8 * wind if strike == 0 else (-0.8 + 2.1 * snap if rec == 0 else 1.3 * out)
        tail = 0.6 * wind if strike == 0 else (0.6 - 1.6 * snap) * out
        p = self.sway(Pose(), self.t(f))
        p.turn("body", E(0, yaw * DEG, 0))
        p.turn("tail.1", E(0, tail * 0.6 * DEG, 0))
        p.turn("tail.2", E(0, tail * DEG, 0))
        return self.neck(p, -0.2 * wind * out, 0.15 * wind * out, 0.3 * snap * out)

    def dive(self, f):
        """Down under the water nose first (the code sinks him and hides him while he is under),
        and up out of it beneath him, the jaws wide."""
        s = ENVY["dive"][0]
        _, _, rec, snap = phases(f, ENVY["dive"])
        p = self.sway(Pose(), self.t(f))
        if f < s:
            u = min(1.0, f / SINK)
            p.turn("body", E(0.4 * u * DEG, 0, 0))
            return self.neck(p, 0.3 * u, 0.2 * u, 0.0)
        if rec == 0:
            return self.neck(p, -0.3 * (1 - snap), -0.2, 0.5)
        return self.neck(p, 0.15 * (1 - rec), 0.1, 0.2 * (1 - rec))

    def bell_toll(self, f):
        """Reared back high, then his head brought down like a beam on the bronze."""
        wind, strike, rec, snap = phases(f, ENVY["bell_toll"])
        out = 1 - rec
        neck = -0.6 * wind if strike == 0 else (-0.6 + 1.3 * snap if rec == 0 else 0.7 * out)
        p = self.sway(Pose(), self.t(f))
        p["scale"] = {"body": (1, 1 + 0.08 * wind * out, 1)}
        return self.neck(p, neck, -0.2 * wind * out, 0.2 * snap * out)

    def burning_coil(self, f):
        """His coils drawn tight about the bell, his head up, the jaw open on the heat."""
        wind, _, rec, _ = phases(f, ENVY["burning_coil"])
        k = wind * (1 - rec)
        p = self.sway(Pose(), self.t(f))
        p["scale"] = {"body": (1 + 0.07 * k, 1 - 0.08 * k, 1 + 0.07 * k)}
        for b, side in (("fin.L", 1.0), ("fin.R", -1.0)):
            p.turn(b, E(0, 0, side * (0.5 * k + math.sin(9 * f / 60.0) * 0.1 * k) * DEG))
        return self.neck(p, -0.25 * k, -0.3 * k, 0.6 * k)

    def roar(self, f):
        """As a phase begins: reared back, fins spread, the jaws wide."""
        wind, _, rec, _ = phases(f, ENVY["roar"])
        k = wind * (1 - rec)
        p = self.sway(Pose(), self.t(f))
        for b, side in (("fin.L", 1.0), ("fin.R", -1.0)):
            p.turn(b, E(0, 0, side * 0.45 * k * DEG))
        return self.neck(p, -0.35 * k, -0.45 * k, 0.75 * k + 0.08 * math.sin(f * 0.9) * k)

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.sway(Pose(), self.t(f))
        p.turn("body", E(-0.08 * k * DEG, 0, 0.05 * k * DEG))
        return self.neck(p, -0.25 * k, -0.2 * k, 0.15 * k)

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        p = self.sway(Pose(), self.t(f) * 3, moving=True)
        p.turn("body", E(0.06 * k * DEG, 0, 0.1 * math.sin(TAU * f / 24) * k * DEG))
        return self.neck(p, 0.3 * k, 0.3 * k, 0.3 * k)

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        return self.neck(self.sway(Pose(), self.t(f)), -0.4 * k, -0.2 * k, 0.0)

    def make(self, arm):
        bake(arm, "idle", 420, self.idle, loop=True)
        bake(arm, "slither", 90, self.slither, loop=True, stride=1.6)
        for name, fd in ENVY.items():
            bake(arm, name, sum(fd), getattr(self, name), marks=move_marks(fd))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


class Serpentling:
    """A sea krait reared off its coil: swaying, its head weaving; moving, it slides side to side; its
    bite drawn back and struck, as the cobra's."""

    def sway(self, p, t, k=0.08):
        z = math.sin(t) * k
        for b, s in zip(NECKS, SHARE):
            p.turn(b, E(0, 0, z * s * DEG))
        p.turn("head", E(math.sin(t * 0.5) * 0.05 * DEG, 0, -math.sin(t) * 0.1 * DEG))
        p.turn("tail", E(0, math.sin(t * 1.9) * 0.35 * DEG, 0))
        return p

    def idle(self, f):
        return self.sway(Pose(), f / 60.0 * 1.6)

    def slither(self, f):
        t = TAU * f / 60
        p = self.sway(Pose(), t, 0.14)
        p.turn("coil", E(0, math.sin(t) * 0.12 * DEG, 0))
        return p

    def bite(self, f):
        s, a, r = BITE
        back = env(f, [(0, 0.0), (s, 1.0), (s + 1, 0.4), (s + 3, 0.0)])
        out = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 2, 1.0), (s + a + r, 0.0)])
        p = self.sway(Pose(), f / 60.0 * 1.6, 0.08 * (1 - max(back, out)))
        for b, k in zip(NECKS, (1.0, 1.2, 1.4)):
            p.turn(b, E((-10 * back + 14 * out) * k, 0, 0))
        p.turn("head", E(14 * back - 18 * out, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.sway(Pose(), f / 60.0 * 6, 0.08 + 0.1 * k)
        p.turn("head", E(-20 * k, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.sway(Pose(), f / 60.0 * 1.6)
        for b in NECKS:
            p.turn(b, E(-6 * k, 0, 4 * k))
        p.turn("head", E(-15 * k, 20 * k, 0))
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        p = self.sway(Pose(), f / 60.0 * 8, 0.08 + 0.15 * k)
        p.turn("head", E(15 * k, 0, 0))
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.sway(Pose(), f / 60.0 * 1.6)
        for b in NECKS:
            p.turn(b, E(-8 * k, 0, 0))
        p.turn("head", E(-15 * k, 0, 0))
        return p

    def make(self, arm):
        bake(arm, "idle", 240, self.idle, loop=True)
        bake(arm, "slither", 60, self.slither, loop=True, stride=1.0)
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


FOES = {"leviathan": Leviathan(), "serpentling": Serpentling()}
