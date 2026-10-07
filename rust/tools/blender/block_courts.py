"""The courts' keyed bones (`mixamo_foe.py` lays them under the princes' Mixamo clips, `lay`; the
beasts' whole clips are baked by `block_foe.py`). Each shows what the code's pose shows
(`view/court_*.rs`, `view/duel.rs`), at the move's frames.

Lucifer's wings (`LuciferWings`): lifted and lowered as he breathes, lifted with his cuts and high
as the stars fall, folded round him as he steps into the light, beating as he hangs over his mark;
and, where his clip comes from the magic pack, his sword arm held as sculpted (the sword point-down
before him).
"""
import math

from block_foe import E, Pose, smooth

# The princes' moves at their base tempo (startup, active, recovery: logic/combat/frames.rs).
FD = {
    "radiant_cut": (32, 8, 42), "lance_of_dawn": (52, 10, 40), "mirror_step": (36, 1, 72), "starfall": (40, 1, 36),
    "prides_fall": (56, 6, 54), "roar": (40, 1, 36),
}
DEG = 180.0 / math.pi


def phases(f, fd):
    """(wind, strike, rec, snap) at frame `f` of a move of frame data `fd`, as the code reckons them."""
    s, a, r = fd
    if f < s:
        wind, strike, rec = smooth(f / s), 0.0, 0.0
    elif f < s + a:
        wind, strike, rec = 1.0, min(1.0, (f - s + 1) / a), 0.0
    else:
        wind, strike, rec = 1.0, 1.0, smooth((f - s - a) / r)
    return wind, strike, rec, 1 - (1 - strike) ** 3


class LuciferWings:
    """His wings (`wings.L`, `wings.R`, and the burnt `burnt.L`, `burnt.R`) as `court_pride::dress_lucifer`
    poses them: each turned (-0.05 fold, -side 1.1 fold, side lift) radians, `side` +1 at +x."""

    held = ["upper_arm.L", "forearm.L", "hand.L", "offhand"]

    def __init__(self, held=None):
        if held is not None:
            self.held = held

    def wings(self, p, lift, fold=0.0):
        for names, side in ((("wings.L", "burnt.L"), 1.0), (("wings.R", "burnt.R"), -1.0)):
            for b in names:
                p.turn(b, E(-0.05 * fold * DEG, -side * 1.1 * fold * DEG, side * lift * DEG))
        return p

    def breath(self, f):
        return 0.04 * math.sin(1.3 * f / 60.0)

    def idle(self, f):
        return self.wings(Pose(), self.breath(f))

    def walk(self, f):
        p = self.wings(Pose(), self.breath(f))
        # (The sword arm swings a little with his stride; the fist keeps the sword point-down.)
        swing = 6.0 * math.sin(2 * math.pi * f / 1000)
        p.turn(self.held[0], E(-swing, 0, 0))
        p.turn(self.held[2], E(swing, 0, 0))
        return p

    def radiant_cut(self, f):
        wind, _, rec, _ = phases(f, FD["radiant_cut"])
        return self.wings(Pose(), self.breath(f) + 0.25 * wind * (1 - rec))

    def lance_of_dawn(self, f):
        wind, _, rec, _ = phases(f, FD["lance_of_dawn"])
        return self.wings(Pose(), self.breath(f) + 0.45 * wind * (1 - rec))

    def mirror_step(self, f):
        wind, _, rec, _ = phases(f, FD["mirror_step"])
        return self.wings(Pose(), self.breath(f), 1.0 * wind * (1 - rec))

    def starfall(self, f):
        wind, _, rec, _ = phases(f, FD["starfall"])
        return self.wings(Pose(), self.breath(f) + 0.6 * wind * (1 - rec))

    def prides_fall(self, f):
        s, a, r = FD["prides_fall"]
        rise, dive = 12, s - 8
        _, _, rec, _ = phases(f, FD["prides_fall"])
        if f < s:
            flap = 0.4 * math.sin(14 * f / 60.0) if rise <= f < dive else 0.0
            lift = 0.6 * smooth(f / rise) + flap
        else:
            lift = 0.3 * (1 - rec)
        return self.wings(Pose(), lift)

    def roar(self, f):
        wind, _, rec, _ = phases(f, FD["roar"])
        return self.wings(Pose(), self.breath(f) + 0.5 * wind * (1 - rec))

    def reel(self, f):
        return self.wings(Pose(), self.breath(f) + 0.15)


class ReflectionWings(LuciferWings):
    """A Reflection's two white sixes (no burnt ones); its sword arm the right."""

    held = ["upper_arm.R", "forearm.R", "hand.R", "blade"]


class AmonTail:
    """Amon's serpent's tail swaying behind him (the code swings it as he walks), streaming straight
    out behind as he charges; with his axe arm held as sculpted where the creature pack's arms swing
    empty (his stand, his walk, his run)."""

    held = ["upper_arm.L", "forearm.L", "hand.L", "offhand"]

    def tail(self, p, f, k=0.3, period=140.0):
        a = k * math.sin(2 * math.pi * f / period)
        p.turn("tail.1", E(0, a * 0.6 * DEG, 0))
        p.turn("tail.2", E(0, a * DEG, 0))
        return p

    def arm(self, p, swing):
        p.turn(self.held[0], E(-swing, 0, 0))
        p.turn(self.held[2], E(swing, 0, 0))
        return p

    def idle(self, f):
        return self.arm(self.tail(Pose(), f), 1.5 * math.sin(2 * math.pi * f / 240))

    def walk(self, f):
        return self.arm(self.tail(Pose(), f, 0.3, 500.0), 8.0 * math.sin(2 * math.pi * f / 1000))

    def run(self, f):
        p = Pose()
        p.turn("tail.1", E(-0.6 * DEG, 0.08 * math.sin(2 * math.pi * f / 250) * DEG, 0))
        p.turn("tail.2", E(-0.2 * DEG, 0.15 * math.sin(2 * math.pi * f / 250) * DEG, 0))
        return self.arm(p, 12.0 * math.sin(2 * math.pi * f / 1000))

    def sway(self, f):
        return self.tail(Pose(), f)


class Still:
    """Bones held as sculpted under a clip (Belphegor's hourglass hanging from his fist)."""

    def still(self, f):
        return Pose()


class NightmareWisps:
    """A nightmare's two wisps (on the legs' bones) trailing below it, swaying out of time with
    each other; as it drifts they stream back."""

    legs = ("thigh", "shin", "foot")

    def wisps(self, f, back=0.0, k=1.0):
        p = Pose()
        for side, ph in (("L", 0.0), ("R", 0.37)):
            for i, b in enumerate(self.legs):
                a = math.sin(2 * math.pi * (f / 150.0 + ph - i * 0.12))
                p.turn(f"{b}.{side}", E((-back * (0.5 + 0.25 * i) + 6 * k * a) , 0, 5 * k * a * (1 if side == "L" else -1)))
        return p

    def idle(self, f):
        return self.wisps(f)

    def walk(self, f):
        return self.wisps(f * 1.6, back=-18.0, k=1.3)

    def move(self, f):
        return self.wisps(f * 1.4, back=-6.0, k=1.5)


FOES = {}


def _raven():
    from block_foe import Flyer

    # Amon's ravens: great black birds, slow heavy beats.
    return Flyer(beat=30, amp=46, bob=0.04, stride=1.3)


FOES["raven"] = _raven()
