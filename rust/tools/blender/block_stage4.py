"""The fourth stage's keyed clips (`block_foe.py` runs them: `block_foe.py -- kappa ...`): the kappa (all
of it: its squat and its arms thrust forward are its own, which Mixamo's would hang at its sides),
the rokurokubi (kneeling, her neck in six), the kitsune (on the wolves' bones, its tail in three), and
the oni's bespoke moves over his Mixamo clips (`mixamo_foe.py` first). Each folklore move shows what
the code's pose shows (`view/duel.rs`), on more joints.
"""
import math

from block_foe import BITE, E, RALLY, TAU, Pose, Wolf, bake, env, move_marks, smooth

SHOVE = (22, 6, 28)
NECK_BITE = (26, 6, 30)
FOXFIRE = (34, 1, 32)


class Biped:
    """A humanoid keyed by angles: the heroes' bone names, `.L` at +x."""

    LEGS = ("thigh", "shin", "foot")

    @staticmethod
    def crouch(p, k, deg=32.0, drop=0.08):
        """Squatting `k` of the way: the thighs forward, the knees bent, the feet flat, the hips
        lowered `drop` metres."""
        for s in ("L", "R"):
            p.turn(f"thigh.{s}", E(-deg * k, 0, 0))
            p.turn(f"shin.{s}", E(2 * deg * k, 0, 0))
            p.turn(f"foot.{s}", E(-deg * k, 0, 0))
        loc = p.get("loc", (0, 0, 0))
        p["loc"] = (loc[0], loc[1] - drop * k, loc[2])
        return p

    @staticmethod
    def step(p, f, n, amp=20.0, lift=30.0, roll=0.0, bob=0.0):
        """A walking cycle: each leg forward then back, the knee bent as it swings through, the
        hips rolling over the standing leg and bobbing twice a cycle."""
        for s, ph in (("L", 0.0), ("R", 0.5)):
            u = (f / n + ph) % 1.0
            if u < 0.5:
                swing, bend = -amp + 2 * amp * (u / 0.5), 0.0
            else:
                w = (u - 0.5) / 0.5
                swing, bend = amp - 2 * amp * smooth(w), lift * math.sin(math.pi * w)
            p.turn(f"thigh.{s}", E(swing - bend * 0.4, 0, 0))
            p.turn(f"shin.{s}", E(bend, 0, 0))
            p.turn(f"foot.{s}", E(-bend * 0.4, 0, 0))
        p.turn("hips", E(0, 0, roll * math.sin(TAU * f / n)))
        loc = p.get("loc", (0, 0, 0))
        p["loc"] = (loc[0], loc[1] + bob * math.cos(TAU * 2 * f / n), loc[2])
        return p

    @staticmethod
    def breathe(p, f, n=120, k=1.0):
        b = math.sin(TAU * f / (n / 2))
        p.turn("chest", E(-1.5 * k * b, 0, 0))
        p.turn("head", E(2 * k * math.sin(TAU * f / n), 8 * k * math.sin(TAU * (f / n - 0.2)), 0))
        return p


# ------------------------------------------------------------------ the kappa


class Kappa(Biped):
    def stand(self, f=0.0):
        p = Pose()
        self.crouch(p, 0.15)
        p.turn("water", E(2 * math.sin(TAU * f / 50), 0, 2 * math.sin(TAU * f / 37)))
        return p

    def idle(self, f):
        n = 120
        p = self.breathe(self.stand(f), f, n)
        for s, sg in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{s}", E(4 * math.sin(TAU * (f / 60 + 0.2 * sg)), 0, 0))
            p.turn(f"hand.{s}", E(-6 * math.sin(TAU * (f / 40 + 0.3 * sg)), 0, 0))
        return p

    def walk(self, f):
        """The waddle: rolling from foot to foot on its bent legs, its arms held out."""
        n = 36
        p = self.step(self.stand(f), f, n, amp=16, lift=26, roll=11, bob=0.012)
        p["loc"] = (0.03 * math.sin(TAU * f / n), p["loc"][1], 0.0)
        p.turn("chest", E(0, 0, -6 * math.sin(TAU * f / n)))
        p.turn("head", E(4 * math.cos(TAU * 2 * f / n), 0, 4 * math.sin(TAU * f / n)))
        for s, sg in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{s}", E(10 * sg * math.sin(TAU * f / n), 0, 0))
        return p

    def shove(self, f):
        """Down in a sumo crouch, palms drawn back to its chest; then the shove: lunging in, both
        palms thrust out at him; then back up."""
        s, a, r = SHOVE
        wind = env(f, [(0, 0.0), (s, 1.0), (s + 1, 0.4), (s + a + 4, 0.0)])
        out = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 4, 1.0), (s + a + r, 0.0)])
        p = self.stand(f)
        self.crouch(p, 0.9 * wind + 0.3 * out)
        p["loc"] = (0.0, p["loc"][1], 0.22 * out)
        p.turn("spine", E(10 * wind + 16 * out, 0, 0))
        p.turn("chest", E(4 * wind + 8 * out, 0, 0))
        p.turn("head", E(-10 * wind - 14 * out, 0, 0))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{side}", E(35 * wind - 55 * out, 0, sg * 10 * wind))
            p.turn(f"forearm.{side}", E(-50 * wind + 30 * out, 0, 0))
            p.turn(f"hand.{side}", E(-30 * out, 0, 0))
        # (The stance opened: a foot forward under the thrust, the other driving behind.)
        p.turn("thigh.L", E(-25 * out, 0, 0))
        p.turn("thigh.R", E(22 * out, 0, 0))
        return p

    def deflected(self, f, n=30):
        """Parried, it bows: deep from the waist, its head down (the code spills its dish)."""
        k = env(f, [(0, 0.0), (4, 1.0), (16, 1.0), (n, 0.0)])
        p = self.stand(f)
        p.turn("spine", E(28 * k, 0, 0))
        p.turn("chest", E(22 * k, 0, 0))
        p.turn("head", E(25 * k, 0, 0))
        p.turn("water", E(35 * k, 0, 0))
        for side in ("L", "R"):
            p.turn(f"upper_arm.{side}", E(30 * k, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.stand(f)
        p.turn("spine", E(-14 * k, 0, 6 * k))
        p.turn("head", E(-18 * k, 0, 0))
        for side in ("L", "R"):
            p.turn(f"upper_arm.{side}", E(-25 * k, 0, 0))
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        sway = math.sin(TAU * f / 22)
        p = self.stand(f)
        p.turn("hips", E(0, 0, 12 * sway * k))
        p.turn("spine", E(10 * k, 0, -10 * sway * k))
        p.turn("head", E(20 * k, 15 * sway * k, 0))
        self.crouch(p, 0.4 * k)
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.stand(f)
        p.turn("chest", E(-15 * k, 0, 0))
        p.turn("head", E(-25 * k, 0, 0))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{side}", E(-60 * k, 0, sg * 30 * k + 6 * math.sin(TAU * f / 8) * k))
        return p

    def make(self, arm):
        bake(arm, "idle", 120, self.idle, loop=True)
        bake(arm, "walk", 36, self.walk, loop=True, stride=0.45)
        bake(arm, "shove", sum(SHOVE), self.shove, marks=move_marks(SHOVE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 30, self.deflected, marks={"recover_end": 30})


# ------------------------------------------------------------------ the rokurokubi

NECKS = ["neck.1", "neck.2", "neck.3", "neck.4", "neck.5", "neck.6"]


class Rokurokubi:
    """She kneels still; her neck weaves, each bone a little later than the one below it, her head
    turning on it."""

    def weave(self, p, f, period, amp, lag=0.1, nod=0.0):
        total = 0.0
        for i, b in enumerate(NECKS):
            a = amp * math.sin(TAU * (f / period - i * lag))
            total += a
            p.turn(b, E(nod * math.sin(TAU * (f / period - i * lag + 0.25)), 0, a))
        p.turn("head", E(0, 12 * math.sin(TAU * f / (period * 1.3)), -total * 0.5))
        return p

    def base(self, f, k=1.0):
        p = Pose()
        p.turn("chest", E(1.5 * math.sin(TAU * f / 70), 0, 0))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn(f"upper_arm.{side}", E(2 * math.sin(TAU * (f / 70 + 0.3 * sg)), 0, 0))
        return p

    def idle(self, f):
        return self.weave(self.base(f), f, 150, 2.6, nod=1.5)

    def walk(self, f):
        """Shuffling on her knees, her neck weaving faster."""
        n = 48
        p = self.weave(self.base(f), f, n, 4.0, lag=0.13, nod=2.0)
        p.turn("body", E(0, 0, 4 * math.sin(TAU * f / n)))
        p["loc"] = (0.0, 0.006 * abs(math.sin(TAU * f / n)), 0.0)
        return p

    def neck_bite(self, f):
        """Her neck draws back, then bows forward a little way and her head bites, jaws wide and
        snapping; it stays on her neck, and her neck its own length."""
        s, a, r = NECK_BITE
        back = env(f, [(0, 0.0), (s, 1.0), (s + 2, 0.0)])
        out = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 3, 1.0), (s + a + r, 0.0)])
        p = self.weave(self.base(f), f, 150, 2.6 * (1 - max(back, out)))
        for i, b in enumerate(NECKS):
            p.turn(b, E(-4 * back * (1 + i * 0.25) + 7 * out * (1.2 - i * 0.12), 0, 0))
        p.turn("head", E(-14 * back + 18 * out, 0, 0))
        p.turn("jaw", E(env(f, [(s - 4, 0.0), (s + 1, 32.0), (s + 3, 32.0), (s + 5, 0.0), (s + a + r, 0.0)]), 0, 0))
        p.turn("chest", E(-3 * back + 4 * out, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.weave(self.base(f), f, 30, 5.0 * k)
        for b in NECKS:
            p.turn(b, E(-3 * k, 0, 0))
        p.turn("head", E(-20 * k, 0, 0))
        p.turn("jaw", E(env(f, [(s - 10, 0.0), (s, 28.0), (n - 14, 24.0), (n - 4, 0.0)]), 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.base(f)
        for i, b in enumerate(NECKS):
            p.turn(b, E(-3 * k, 0, 3 * k * (1 if i % 2 else -1)))
        p.turn("head", E(-15 * k, 20 * k, 0))
        p.turn("chest", E(-6 * k, 0, 0))
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        p = self.weave(self.base(f), f, 18, 8.0 * k, lag=0.18)
        p.turn("head", E(15 * k, 0, 0))
        p.turn("jaw", E(15 * k, 0, 0))
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.base(f)
        for b in NECKS:
            p.turn(b, E(-5 * k, 0, 0))
        p.turn("head", E(-18 * k, 0, 0))
        p.turn("jaw", E(20 * k, 0, 0))
        return p

    def make(self, arm):
        bake(arm, "idle", 150, self.idle, loop=True)
        bake(arm, "walk", 48, self.walk, loop=True, stride=0.5)
        bake(arm, "neck_bite", sum(NECK_BITE), self.neck_bite, marks=move_marks(NECK_BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


# ------------------------------------------------------------------ the kitsune


class Kitsune(Wolf):
    """The wolves' gaits and reels, lighter; its tail raised in its S, and its foxfire."""

    def stand(self, f=0.0):
        p = Pose()
        p.turn("tail.1", E(-4 + 3 * math.sin(TAU * f / 80), 5 * math.sin(TAU * f / 80), 0))
        p.turn("tail.2", E(3 * math.sin(TAU * (f / 80 - 0.15)), 6 * math.sin(TAU * (f / 80 - 0.1)), 0))
        p.turn("tail.3", E(4 * math.sin(TAU * (f / 80 - 0.3)), 8 * math.sin(TAU * (f / 80 - 0.2)), 0))
        return p

    def foxfire(self, f):
        """Its head lifts and its tail draws back as the foxfire gathers over its tip; then the
        tail flicks forward and the fire is loosed; then it settles."""
        s, a, r = FOXFIRE
        n = s + a + r
        k = env(f, [(0, 0.0), (s, 1.0), (s + 1, 1.0), (n, 0.0)])
        flick = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + 12, 0.6), (n, 0.0)])
        p = self.stand(f)
        p["loc"] = (0.0, -0.02 * k, -0.04 * k)
        p.turn("hips", E(-4 * k, 0, 0))
        p.turn("neck", E(-20 * k + 10 * flick, 0, 0))
        p.turn("head", E(-12 * k + 8 * flick, 0, 0))
        p.turn("jaw", E(10 * k, 0, 0))
        p.turn("tail.1", E(-14 * k + 22 * flick, 0, 0))
        p.turn("tail.2", E(-8 * k + 14 * flick, 0, 0))
        p.turn("tail.3", E(-6 * k + 16 * flick, 0, 0))
        for side in ("L", "R"):
            self.leg(p, "f", side, -8 * k, 6 * k)
            self.leg(p, "b", side, -10 * k, 18 * k)
        return p

    def make(self, arm):
        bake(arm, "idle", 120, self.idle, loop=True)
        bake(arm, "walk", self.trot, self.trot_pose, loop=True, stride=self.strides[0])
        bake(arm, "run", self.gallop, self.gallop_pose, loop=True, stride=self.strides[1])
        bake(arm, "foxfire", sum(FOXFIRE), self.foxfire, marks=dict(move_marks(FOXFIRE), hit=FOXFIRE[0]))
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


FOES = {
    "kappa": Kappa(),
    "rokurokubi": Rokurokubi(),
    "kitsune": Kitsune(trot=30, gallop=22, strides=(0.6, 1.4)),
}
