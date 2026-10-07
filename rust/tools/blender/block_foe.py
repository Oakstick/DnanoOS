"""The clips of the foes Mixamo cannot drive, keyed by script (ANIMATION.md §11): the bats (the raven,
its night kin the vampire, the large bat), the wolves (the loup-garou, the dire wolf), the cobra, the
bog slime, the witch's minion, and the jorogumo's eight legs (her upper body is Mixamo's,
`mixamo_foe.py`, laid over these afterwards).

    blender -b -P tools/blender/block_foe.py -- FOE FOE_rig.glb FOE.blend

FOE is the rig's name (`bat`, `vampire`, `large_bat`, `werewolf`, `dire_wolf`, `cobra`, `slime`,
`minion`, `jorogumo`), FOE_rig.glb what `--export-rig FOE` wrote. Every clip is a function of the
frame, baked on every frame at 60 fps (so what plays is exactly what is written here), into the
.blend; then `preview.py` and `export_clips.py` as for any clip.

The clips each foe gets, by the names the game's animator asks for (`anim::foe`):

* its stand: `idle` (`hover` for a flyer), looping;
* its gait: `walk` (`fly` for a flyer, `slither` for the cobra), looping, and `run` where it runs
  (the wolves' gallop), each with the ground a cycle covers (`vh_stride`);
* its moves, keyed to their frame data at the base tempo (`windup_end` on the startup's last
  frame, `active_end` on the active frames' last, `recover_end` at the end: the game warps them
  onto the move as it is swung): `bite` (18, 6, 24 frames), `hex` (36, 1, 30) and the pack
  leader's `rally` (40, 1, 36);
* its reels: `hit` (a flinch), `stagger` (a broken poise or guard) and `deflected` (a parried bite).

Angles are degrees about the game's axes (x pitches: a hanging limb swings back, the body leans
forward; y turns toward its left; z tips: a limb out to its left rises), relative to the bone's
parent; places in metres. Each foe's bones are named by the side they lie on (`.L` at +x).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import vh_blender as vh  # noqa: E402
from mathutils import Quaternion, Vector  # noqa: E402

TAU = 2 * math.pi
X, Y, Z = Vector((1, 0, 0)), Vector((0, 1, 0)), Vector((0, 0, 1))

# Frame data at the base tempo (logic/combat/frames.rs): startup, active, recovery.
BITE = (18, 6, 24)
HEX = (36, 1, 30)
RALLY = (40, 1, 36)


def E(x=0.0, y=0.0, z=0.0):
    """A turn of x, y, z degrees about the game's axes (the engine's XYZ)."""
    return vh.quat_g(x, y, z)


def A(axis, deg):
    return vh.axis_quat(axis, deg)


def smooth(u):
    u = max(0.0, min(1.0, u))
    return u * u * (3 - 2 * u)


def env(f, keys, ease=True):
    """The value at frame `f` along keys [(frame, value), ...], eased between them."""
    if f <= keys[0][0]:
        return keys[0][1]
    for (f0, v0), (f1, v1) in zip(keys, keys[1:]):
        if f <= f1:
            u = (f - f0) / max(1e-6, f1 - f0)
            u = smooth(u) if ease else u
            return v0 + (v1 - v0) * u
    return keys[-1][1]


def snap(u):
    """Ease out, hard: a strike that arrives fast and stops."""
    u = max(0.0, min(1.0, u))
    return 1 - (1 - u) ** 3


def wave(f, period, phase=0.0):
    return math.sin(TAU * (f / period + phase))


class Pose(dict):
    """A pose: bone -> turn (game axes, relative to its parent), plus "loc" (the root's offset) and
    "scale" (bone -> its scale on its own axes, which are the game's)."""

    def turn(self, bone, q):
        self[bone] = q @ self.get(bone, Quaternion())
        return self

    def mix(self, other, u):
        """`u` of the way to `other`, bone by bone."""
        out = Pose()
        for b in set(self) | set(other):
            if b in ("loc", "scale"):
                continue
            out[b] = self.get(b, Quaternion()).slerp(other.get(b, Quaternion()), u)
        la, lb = Vector(self.get("loc", (0, 0, 0))), Vector(other.get("loc", (0, 0, 0)))
        out["loc"] = tuple(la.lerp(lb, u))
        sa, sb = self.get("scale", {}), other.get("scale", {})
        out["scale"] = {b: tuple(Vector(sa.get(b, (1, 1, 1))).lerp(Vector(sb.get(b, (1, 1, 1))), u)) for b in set(sa) | set(sb)}
        return out


def bake(arm, name, frames, pose_at, loop=False, stride=None, marks=None):
    """Keys `pose_at(f)` on every frame 0..frames of a new action `name`."""
    old = bpy.data.actions.get(name)
    if old:
        bpy.data.actions.remove(old)
    act = vh.new_action(arm, name)
    bones = vh.bones_of(arm)
    root = vh.root_of(arm)
    for f in range(frames + 1):
        p = pose_at(f % frames if loop and f == frames else f)
        scale = p.get("scale", {})
        for b in bones:
            vh.set_local(arm, b, p.get(b, Quaternion()), p.get("loc", (0, 0, 0)) if b == root else None)
            pb = arm.pose.bones[b]
            pb.keyframe_insert("rotation_quaternion", frame=f)
            if b == root:
                pb.keyframe_insert("location", frame=f)
            pb.scale = Vector(scale.get(b, (1, 1, 1)))
            pb.keyframe_insert("scale", frame=f)
    for fc in vh.fcurves(act):
        for kp in fc.keyframe_points:
            kp.interpolation = "LINEAR"
    act.frame_range = (0, frames)
    act.use_frame_range = True
    act["vh_loop"] = loop
    if stride:
        act["vh_stride"] = stride
    for k, v in (marks or {}).items():
        act.pose_markers.new(k).frame = int(v)
    print(f"block_foe: {name:10} {frames:4} frames{' loop' if loop else ''}{f', stride {stride} m' if stride else ''} {marks or ''}")
    return act


def move_marks(fd):
    s, a, r = fd
    return {"windup_end": s, "active_end": s + a, "recover_end": s + a + r}


# ------------------------------------------------------------------ the flyers


class Flyer:
    """A bat or a raven: `beat` frames a wingbeat, `amp` degrees of it, `bob` metres of lift."""

    def __init__(self, beat, amp, bob, bank=0.0, wrap=False, stride=1.0):
        self.beat, self.amp, self.bob, self.bank, self.wrap, self.stride = beat, amp, bob, bank, wrap, stride

    def wings(self, p, w, fold, sweep=0.0):
        """Both wings at `w` degrees up (negative down), their hands folded `fold` degrees back and
        down, the whole wing swept `sweep` degrees forward."""
        p.turn("wing.L", E(0, -sweep, w))
        p.turn("wing.R", E(0, sweep, -w))
        p.turn("hand.L", E(0, fold, -fold * 0.6))
        p.turn("hand.R", E(0, -fold, fold * 0.6))
        return p

    def stroke(self, ph):
        """The wing's angle and the hand's fold at `ph` (0..1) through a beat: a snapping downstroke
        in its first third, a slower upstroke with the hand folded."""
        a = self.amp
        if ph < 0.32:
            u = ph / 0.32
            return a - (a * 1.6) * (u * u * (3 - 2 * u)) ** 0.8, 0.0
        u = (ph - 0.32) / 0.68
        return -0.6 * a + 1.6 * a * smooth(u), 38 * math.sin(math.pi * u)

    def beat_pose(self, f, beat, pitch=0.0, amp_k=1.0):
        ph = (f / beat) % 1.0
        w, fold = self.stroke(ph)
        p = Pose()
        p["loc"] = (0.0, self.bob * math.sin(TAU * (ph - 0.1)), 0.0)
        p.turn("body", E(pitch + 4 * math.sin(TAU * ph), 0, 0))
        # (The head kept level against the body's rocking; the tail trims.)
        p.turn("head", E(-pitch * 0.6 - 4 * math.sin(TAU * ph), 0, 0))
        p.turn("tail", E(-6 * math.sin(TAU * (ph + 0.15)), 0, 0))
        p.turn("foot.L", E(35, 0, 0))
        p.turn("foot.R", E(35, 0, 0))
        return self.wings(p, w * amp_k, fold)

    def hover(self, f):
        n = self.hover_frames()
        p = self.beat_pose(f, self.beat, pitch=-4)
        if self.bank:
            p.turn("body", E(0, 0, self.bank * 0.4 * math.sin(TAU * f / n)))
        if self.wrap:
            # The night raven folds its wings about itself like a cape between two beats, sinking
            # a little, and spreads them again.
            k = env(f, [(2 * self.beat - 6, 0.0), (2 * self.beat + 8, 1.0), (n - 10, 1.0), (n, 0.0)])
            wrap = Pose()
            wrap["loc"] = (0.0, -0.05, 0.0)
            wrap.turn("body", E(-8, 0, 0))
            wrap.turn("foot.L", E(35, 0, 0)).turn("foot.R", E(35, 0, 0))
            self.wings(wrap, -35, 70, sweep=55)
            wrap.turn("head", E(4 * math.sin(TAU * f / 40), 18 * math.sin(TAU * f / n), 0))
            p = p.mix(wrap, k)
        return p

    def hover_frames(self):
        return self.beat * (3 if self.wrap else 2)

    def fly(self, f):
        """Leaning into its flight: two quick beats and a glide, wings held up a little."""
        b = int(self.beat * 0.8)
        n = 3 * b
        if f < 2 * b:
            p = self.beat_pose(f, b, pitch=18, amp_k=1.1)
        else:
            u = (f - 2 * b) / b
            p = self.beat_pose(0, b, pitch=18)
            g = Pose()
            g["loc"] = (0.0, -self.bob * 0.5 * math.sin(math.pi * u), 0.0)
            g.turn("body", E(16, 0, 0)).turn("head", E(-10, 0, 0)).turn("tail", E(-4, 0, 0))
            g.turn("foot.L", E(40, 0, 0)).turn("foot.R", E(40, 0, 0))
            self.wings(g, 12, 0)
            p = g.mix(p, smooth(u * 1.4 - 0.4))
        if self.bank:
            p.turn("body", E(0, 0, self.bank * math.sin(TAU * f / n)))
        return p

    def bite(self, f):
        """The dive: up high on its wings, nose up, head drawn back; then down at him, jaws wide
        and snapping; then beating back up."""
        s, a, r = BITE
        rest = self.beat_pose(0, self.beat, pitch=-4)
        p = Pose()
        rise = env(f, [(0, 0.0), (s, 1.0), (s + 1, 1.0), (s + a, 0.0)])
        dive = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 6, 1.0), (s + a + r, 0.0)])
        p["loc"] = (0.0, 0.16 * rise - 0.14 * dive, 0.32 * dive)
        p.turn("body", E(-28 * rise + 50 * dive, 0, 0))
        p.turn("head", E(-22 * rise + 12 * dive, 0, 0))
        p.turn("tail", E(15 * rise - 20 * dive, 0, 0))
        jaw = env(f, [(s - 4, 0.0), (s + 1, 40.0), (s + 3, 40.0), (s + 5, 0.0), (s + a + r, 0.0)])
        p.turn("jaw", E(jaw, 0, 0))
        p.turn("foot.L", E(35 - 50 * dive, 0, 0)).turn("foot.R", E(35 - 50 * dive, 0, 0))
        # Wings: raised and swept back in the wind-up (a flutter), slammed down in the dive.
        flutter = 10 * math.sin(TAU * f / 8) * rise
        w = 62 * rise - 45 * dive + flutter
        self.wings(p, w, 30 * rise, sweep=-25 * rise + 20 * dive)
        back = env(f, [(s + a + 4, 0.0), (s + a + r, 1.0)])
        if back > 0:
            p = p.mix(self.beat_pose(f - s - a, self.beat * 0.8, pitch=-4), back * 0.6).mix(rest, smooth((f - s - a - 12) / 12))
        return p

    def rally(self, f):
        """The screech: wings thrown wide and quivering, head up, beak open."""
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s, 1.0), (s + 20, 1.0), (n, 0.0)])
        p = self.beat_pose(f, self.beat, pitch=-4).mix(Pose(), k)
        p["loc"] = (0.0, 0.12 * k, 0.0)
        p.turn("body", E(-22 * k, 0, 0))
        p.turn("head", E(-30 * k + 3 * math.sin(TAU * f / 5) * k, 0, 0))
        p.turn("jaw", E(env(f, [(s - 10, 0.0), (s, 45.0), (s + 22, 40.0), (n - 6, 0.0)]), 0, 0))
        quiver = 4 * math.sin(TAU * f / 4) * k
        self.wings(p, (30 + quiver) * k, 0, sweep=-10 * k)
        p.turn("foot.L", E(10, 0, 0)).turn("foot.R", E(10, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (8, 0.8), (n, 0.0)])
        p = self.beat_pose(f, self.beat, pitch=-4)
        p["loc"] = (0.0, 0.03 * k, -0.12 * k)
        p.turn("body", E(-28 * k, 0, 8 * k))
        p.turn("head", E(-20 * k, 0, 0))
        self.wings(p, 45 * k, 20 * k)
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 14, 0.8), (n, 0.0)])
        p = self.beat_pose(f, self.beat * 1.4, pitch=-4, amp_k=1 - 0.4 * k)
        p["loc"] = (0.0, -0.14 * k, -0.05 * k)
        p.turn("body", E(10 * k, 0, 32 * math.sin(TAU * f / 22) * k))
        p.turn("head", E(15 * k, 20 * math.sin(TAU * f / 16) * k, 0))
        p.turn("wing.L", E(0, 0, -25 * k * math.sin(TAU * f / 11)))
        p.turn("jaw", E(15 * k, 0, 0))
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.beat_pose(f, self.beat, pitch=-4)
        p["loc"] = (0.0, 0.05 * k, -0.18 * k)
        p.turn("body", E(-35 * k, 0, 0))
        p.turn("head", E(-25 * k, 0, 0))
        self.wings(p, 55 * k, 10 * k, sweep=15 * k)
        return p

    def make(self, arm):
        bake(arm, "hover", self.hover_frames(), self.hover, loop=True)
        bake(arm, "fly", int(self.beat * 0.8) * 3, self.fly, loop=True, stride=self.stride)
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


# ------------------------------------------------------------------ the wolves


class Wolf:
    """A wolf: `trot` and `gallop` frames a cycle, its strides (metres a cycle) for each."""

    LEGS = (("f", "L"), ("f", "R"), ("b", "L"), ("b", "R"))

    def __init__(self, trot, gallop, strides, heavy=1.0):
        self.trot, self.gallop, self.strides, self.heavy = trot, gallop, strides, heavy

    @staticmethod
    def leg(p, end, side, swing, bend, paw=0.0):
        """A leg `swing` degrees back (negative: reaching forward), its lower leg folded `bend`
        degrees back (the carpus or the hock flexing), its paw `paw` more."""
        p.turn(f"upper_{end}.{side}", E(swing, 0, 0))
        p.turn(f"lower_{end}.{side}", E(bend, 0, 0))
        p.turn(f"paw_{end}.{side}", E(paw, 0, 0))

    def cycle(self, p, f, n, phases, amp, stance, lift):
        """Every leg through its step: on the ground (`stance` of the cycle) it sweeps from forward to
        back; in the air it comes forward again, folded and lifted."""
        for (end, side) in self.LEGS:
            ph = (f / n - phases[end + side]) % 1.0
            if ph < stance:
                swing = -amp + 2 * amp * (ph / stance)
                bend, paw = 0.0, -10 * (ph / stance)
            else:
                u = (ph - stance) / (1 - stance)
                swing = amp - 2 * amp * smooth(u)
                bend = lift * math.sin(math.pi * u)
                paw = lift * 0.6 * math.sin(math.pi * u)
            # (The hind leg's hock folds forward under it, the foreleg's carpus back.)
            self.leg(p, end, side, swing, bend if end == "f" else -bend * 0.8, paw)
        return p

    def stand(self, f=0.0):
        p = Pose()
        p.turn("tail.1", E(-10, 6 * math.sin(TAU * f / 90), 0))
        p.turn("tail.2", E(10, 8 * math.sin(TAU * (f / 90 - 0.1)), 0))
        p.turn("tail.3", E(8, 10 * math.sin(TAU * (f / 90 - 0.2)), 0))
        return p

    def idle(self, f):
        n = 120
        p = self.stand(f)
        breath = math.sin(TAU * f / 60)
        p["loc"] = (0.0, 0.004 * breath, 0.0)
        p.turn("chest", E(1.5 * breath, 0, 0))
        p.turn("neck", E(-2 * breath + 4, 14 * math.sin(TAU * f / n), 0))
        p.turn("head", E(4 * math.sin(TAU * f / n * 2), 6 * math.sin(TAU * (f / n - 0.15)), 0))
        p.turn("jaw", E(4 + 3 * max(0.0, breath), 0, 0))
        return p

    def trot_pose(self, f):
        """The trot: the legs in diagonal pairs, the back level, the head steady."""
        n = self.trot
        p = self.cycle(self.stand(f), f, n, {"fL": 0.0, "bR": 0.0, "fR": 0.5, "bL": 0.5}, 24, 0.58, 38)
        p["loc"] = (0.0, 0.012 * math.cos(TAU * 2 * f / n), 0.0)
        p.turn("hips", E(0, 0, 3 * math.sin(TAU * f / n)))
        p.turn("chest", E(0, 0, -3 * math.sin(TAU * f / n)))
        p.turn("neck", E(-2 * math.cos(TAU * 2 * f / n), 0, 0))
        p.turn("tail.1", E(0, 10 * math.sin(TAU * f / n), 0))
        return p

    def gallop_pose(self, f):
        """The rotary gallop: the hind legs then the forelegs strike one after the other; the spine
        gathers as the hind legs come under and stretches as they drive; the body rises in the
        suspension; the head stays steady against it all."""
        n = self.gallop
        ph = {"bL": 0.0, "bR": 0.1, "fR": 0.42, "fL": 0.52}
        p = self.cycle(self.stand(f), f, n, ph, 38, 0.4, 62)
        flex = math.sin(TAU * (f / n - 0.15))
        p["loc"] = (0.0, 0.035 * self.heavy * math.sin(TAU * (f / n - 0.3)), 0.0)
        p.turn("hips", E(10 * flex, 0, 0))
        p.turn("chest", E(-16 * flex, 0, 0))
        p.turn("neck", E(8 * flex + 6, 0, 0))
        p.turn("head", E(-2 * flex, 0, 0))
        p.turn("tail.1", E(-14 - 10 * flex, 0, 0))
        p.turn("tail.2", E(-6 * flex, 0, 0))
        p.turn("jaw", E(14, 0, 0))
        return p

    def bite(self, f):
        """Hackles up, crouched on its haunches, head drawn back low; then the lunge, forelegs
        reaching, the jaws flung wide and snapping shut on him; then back on its feet."""
        s, a, r = BITE
        crouch = env(f, [(0, 0.0), (s, 1.0), (s + 1, 0.6), (s + a + 4, 0.0)])
        lunge = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 3, 1.0), (s + a + r, 0.0)])
        p = self.stand(f)
        p["loc"] = (0.0, -0.06 * crouch * self.heavy + 0.02 * lunge, 0.34 * lunge)
        p.turn("hips", E(-6 * crouch + 4 * lunge, 0, 0))
        p.turn("chest", E(10 * crouch - 8 * lunge, 0, 0))
        p.turn("neck", E(-18 * crouch + 22 * lunge, 0, 0))
        p.turn("head", E(-8 * crouch + 14 * lunge, 0, 0))
        jaw = env(f, [(0, 0.0), (s - 2, 14.0), (s + 1, 42.0), (s + 3, 42.0), (s + 5, 0.0), (s + a + r, 0.0)])
        p.turn("jaw", E(jaw, 0, 0))
        for side in ("L", "R"):
            self.leg(p, "f", side, -18 * crouch - 42 * lunge, 30 * crouch - 10 * lunge, -15 * lunge)
            self.leg(p, "b", side, -25 * crouch + 28 * lunge, 30 * crouch, 10 * lunge)
        p.turn("tail.1", E(30 * crouch - 25 * lunge, 0, 0))
        return p

    def rally(self, f):
        """The howl: down on its haunches, head thrown up, the jaws open and trembling."""
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 10, 1.0), (n - 12, 1.0), (n, 0.0)])
        p = self.stand(f)
        p["loc"] = (0.0, -0.05 * k * self.heavy, -0.03 * k)
        p.turn("hips", E(-18 * k, 0, 0))
        p.turn("chest", E(-8 * k, 0, 0))
        for side in ("L", "R"):
            self.leg(p, "b", side, -40 * k, 50 * k, -20 * k)
            self.leg(p, "f", side, 22 * k, -5 * k)
        p.turn("neck", E(-38 * k, 0, 0))
        p.turn("head", E(-20 * k + 2 * math.sin(TAU * f / 6) * k, 0, 0))
        p.turn("jaw", E(env(f, [(s - 14, 0.0), (s, 28.0), (n - 16, 24.0), (n - 6, 0.0)]) + 2 * math.sin(TAU * f / 5), 0, 0))
        p.turn("tail.1", E(20 * k, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.stand(f)
        p["loc"] = (0.0, 0.01 * k, -0.08 * k)
        p.turn("chest", E(-10 * k, 0, 6 * k))
        p.turn("neck", E(-20 * k, 15 * k, 0))
        p.turn("head", E(-10 * k, 0, 0))
        p.turn("jaw", E(25 * k, 0, 0))
        return p

    def stagger(self, f, n=48):
        """Its poise broken: it stumbles sideways, legs splaying, head swinging."""
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        sway = math.sin(TAU * f / 24)
        p = self.stand(f)
        p["loc"] = (0.05 * sway * k, -0.04 * k, 0.0)
        p.turn("hips", E(0, 0, 12 * sway * k))
        p.turn("chest", E(8 * k, 0, -10 * sway * k))
        p.turn("neck", E(25 * k, 20 * sway * k, 0))
        p.turn("jaw", E(20 * k, 0, 0))
        for end in ("f", "b"):
            self.leg(p, end, "L", 10 * sway * k, 15 * k)
            self.leg(p, end, "R", -10 * sway * k, 15 * k)
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.stand(f)
        p["loc"] = (0.0, 0.03 * k, -0.15 * k)
        p.turn("hips", E(-10 * k, 0, 0))
        p.turn("chest", E(-14 * k, 0, 0))
        p.turn("neck", E(-25 * k, 0, 0))
        p.turn("jaw", E(30 * k, 0, 0))
        for side in ("L", "R"):
            self.leg(p, "f", side, -30 * k, 25 * k)
        return p

    def make(self, arm):
        bake(arm, "idle", 120, self.idle, loop=True)
        bake(arm, "walk", self.trot, self.trot_pose, loop=True, stride=self.strides[0])
        bake(arm, "run", self.gallop, self.gallop_pose, loop=True, stride=self.strides[1])
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


# ------------------------------------------------------------------ the cobra

NECK = ["neck.1", "neck.2", "neck.3", "neck.4", "neck.5"]


class Cobra:
    def sway(self, p, f, period, amp, lag=0.09, yaw=0.0):
        """A wave up the neck: each bone tipped side to side a little later than the one below it,
        the head turned back against it to keep its eyes on him."""
        total = 0.0
        for i, b in enumerate(NECK):
            a = amp * math.sin(TAU * (f / period - i * lag))
            total += a
            p.turn(b, E(0, yaw * math.sin(TAU * (f / period - i * lag - 0.25)), a))
        p.turn("hood", E(0, 0, -total * 0.5))
        p.turn("head", E(0, 0, -total * 0.4))
        return p

    def tail(self, p, f, period=50, amp=18):
        p.turn("tail.1", E(0, amp * 0.5 * math.sin(TAU * f / period), 0))
        p.turn("tail.2", E(0, amp * math.sin(TAU * (f / period - 0.2)), 0))
        return p

    def idle(self, f):
        n = 120
        p = self.tail(self.sway(Pose(), f, n, 3.0), f, 60)
        p.turn("head", E(3 * math.sin(TAU * f / 60), 6 * math.sin(TAU * f / n * 2), 0))
        flick = max(0.0, math.sin(TAU * f / 40)) ** 8
        p.turn("jaw", E(6 * flick, 0, 0))
        return p

    def slither(self, f):
        """Its coil sliding from side to side as it goes, the neck weaving over it."""
        n = 48
        p = self.tail(self.sway(Pose(), f, n, 6.0, lag=0.12, yaw=5.0), f, n / 2, 25)
        p["loc"] = (0.045 * math.sin(TAU * f / n), 0.0, 0.0)
        p.turn("coil", E(0, 9 * math.sin(TAU * (f / n - 0.2)), 3 * math.sin(TAU * f / n)))
        return p

    def bite(self, f):
        """Drawn back on its coil, the hood flared; then the strike down off it onto him, its fangs
        bared; then back up."""
        s, a, r = BITE
        back = env(f, [(0, 0.0), (s, 1.0), (s + 1, 0.4), (s + 3, 0.0)])
        out = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 2, 1.0), (s + a + r, 0.0)])
        p = self.tail(Pose(), f)
        p["loc"] = (0.0, -0.03 * back, 0.16 * out)
        for i, b in enumerate(NECK):
            p.turn(b, E(-6 * back * (1 + i * 0.3) + 13 * out * (1.2 - i * 0.15), 0, 0))
        p.turn("hood", E(12 * back - 12 * out, 0, 0))
        p.turn("head", E(10 * back - 18 * out, 0, 0))
        p["scale"] = {"hood": (1 + 0.28 * max(back, out * 0.6), 1.0, 1.0)}
        jaw = env(f, [(s - 3, 6.0), (s + 1, 50.0), (s + 3, 50.0), (s + 5, 0.0), (s + a + r, 0.0)])
        p.turn("jaw", E(jaw, 0, 0))
        return p

    def rally(self, f):
        """Reared up to its full height, the hood spread, hissing, swaying."""
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.tail(self.sway(Pose(), f, 40, 5.0 * k), f, 30, 28)
        p["loc"] = (0.0, 0.06 * k, -0.02 * k)
        for b in NECK:
            p.turn(b, E(-3 * k, 0, 0))
        p.turn("head", E(10 * k, 0, 0))
        p["scale"] = {"hood": (1 + 0.32 * k, 1.0, 1.0)}
        p.turn("jaw", E(env(f, [(s - 10, 0.0), (s, 32.0), (n - 14, 28.0), (n - 4, 0.0)]), 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.tail(Pose(), f)
        for i, b in enumerate(NECK):
            p.turn(b, E(-4 * k * (i + 1) * 0.6, 0, 3 * k))
        p.turn("head", E(-12 * k, 10 * k, 0))
        p.turn("jaw", E(20 * k, 0, 0))
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        p = self.tail(self.sway(Pose(), f, 20, 9.0 * k, lag=0.15), f, 14, 30)
        for b in NECK:
            p.turn(b, E(5 * k, 0, 0))
        p.turn("head", E(15 * k, 0, 0))
        p.turn("jaw", E(15 * k, 0, 0))
        p["loc"] = (0.0, -0.03 * k, 0.0)
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.tail(Pose(), f)
        for i, b in enumerate(NECK):
            p.turn(b, E(-7 * k, 0, 0))
        p.turn("head", E(-15 * k, 0, 0))
        p.turn("jaw", E(30 * k, 0, 0))
        p["loc"] = (0.0, 0.0, -0.1 * k)
        return p

    def make(self, arm):
        bake(arm, "idle", 120, self.idle, loop=True)
        bake(arm, "slither", 48, self.slither, loop=True, stride=0.9)
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


# ------------------------------------------------------------------ the bog slime


class Slime:
    @staticmethod
    def squash(p, bone, k):
        """`bone` squashed by `k` (negative: stretched), its volume kept."""
        y = 1.0 - k
        xz = 1.0 / math.sqrt(max(0.3, y))
        p.setdefault("scale", {})[bone] = (xz, y, xz)
        return p

    def wobble(self, f, n=90, k=1.0):
        p = Pose()
        self.squash(p, "base", 0.05 * k * math.sin(TAU * f / n * 2))
        self.squash(p, "belly", 0.04 * k * math.sin(TAU * (f / n * 2 - 0.15)))
        self.squash(p, "crown", 0.05 * k * math.sin(TAU * (f / n * 2 - 0.3)))
        p.turn("crown", E(3 * k * math.sin(TAU * f / n), 0, 3 * k * math.sin(TAU * (f / n - 0.25))))
        p.turn("lily", E(6 * k * math.sin(TAU * (f / n - 0.2)), 0, 8 * k * math.sin(TAU * (f / n - 0.45))))
        p.turn("mouth", E(2 + 2 * math.sin(TAU * f / 45), 0, 0))
        for side, ph in (("L", 0.0), ("R", 0.5)):
            p["scale"]["lobe." + side] = (1 + 0.06 * math.sin(TAU * (f / 45 + ph)), 1.0, 1.0)
        return p

    def idle(self, f):
        return self.wobble(f)

    def hop(self, f):
        """A hop: squashed down gathering itself, stretched as it springs, up, and squashed again as
        it lands, the crown and the lily lagging behind it all."""
        n = 36
        up = math.sin(math.pi * min(1.0, max(0.0, (f - 9) / 20)))
        p = Pose()
        sq = env(f, [(0, 0.15), (8, 0.22), (11, -0.2), (18, -0.05), (28, -0.08), (31, 0.25), (n, 0.15)])
        self.squash(p, "base", sq)
        self.squash(p, "crown", -sq * 0.4)
        p["loc"] = (0.0, 0.22 * up, 0.0)
        p.turn("base", E(env(f, [(0, 0.0), (10, 8.0), (20, -4.0), (30, 6.0), (n, 0.0)]), 0, 0))
        p.turn("crown", E(env(f, [(0, -2.0), (12, -9.0), (24, 6.0), (33, 4.0), (n, -2.0)]), 0, 0))
        p.turn("lily", E(env(f, [(0, 0.0), (13, -14.0), (25, 12.0), (34, -4.0), (n, 0.0)]), 0, 0))
        p.turn("mouth", E(4 * up, 0, 0))
        return p

    def hex(self, f):
        """The spit: it swells, rearing back, its belly bulging; then the mouth gapes and it lurches
        forward as the gob flies (the bolt leaves on the move's active frame); then it settles."""
        s, a, r = HEX
        n = s + a + r
        swell = env(f, [(0, 0.0), (s, 1.0), (s + 2, 0.0), (n, 0.0)])
        fire = env(f, [(s - 1, 0.0), (s + 2, 1.0), (s + 10, 0.7), (n, 0.0)])
        p = self.wobble(f, 30, 0.6 * swell + 0.2)
        b = 1 + 0.18 * swell
        p["scale"]["belly"] = (b, b * 1.04, b * 1.12)
        p["scale"]["crown"] = (1 + 0.08 * swell, 1 + 0.06 * swell, 1 + 0.08 * swell)
        p.turn("base", E(-8 * swell + 14 * fire, 0, 0))
        p.turn("crown", E(-10 * swell + 8 * fire, 0, 0))
        p.turn("mouth", E(-4 * swell + 42 * fire, 0, 0))
        self.squash(p, "base", -0.1 * swell + 0.18 * fire)
        p["loc"] = (0.0, 0.0, 0.08 * fire)
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s, 1.0), (n - 10, 0.8), (n, 0.0)])
        p = self.wobble(f, 20, 1.0 + 2.0 * k)
        self.squash(p, "base", -0.2 * k)
        self.squash(p, "belly", -0.15 * k)
        p["loc"] = (0.0, 0.05 * k * abs(math.sin(TAU * f / 20)), 0.0)
        p.turn("mouth", E(30 * k, 0, 0))
        p.turn("lily", E(0, 0, 18 * math.sin(TAU * f / 14) * k))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (2, 1.0), (n, 0.0)])
        p = self.wobble(f, 12, 3 * k)
        self.squash(p, "base", 0.25 * k)
        p.turn("crown", E(-12 * k, 0, 0))
        p["loc"] = (0.0, 0.0, -0.08 * k)
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (4, 1.0), (n - 10, 0.7), (n, 0.0)])
        p = self.wobble(f, 16, 4 * k)
        sway = math.sin(TAU * f / 20)
        p.turn("base", E(0, 0, 12 * sway * k))
        p.turn("crown", E(0, 0, -16 * sway * k))
        self.squash(p, "base", 0.2 * k)
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.wobble(f, 10, 2 * k)
        self.squash(p, "base", -0.15 * k)
        p.turn("base", E(-15 * k, 0, 0))
        p["loc"] = (0.0, 0.0, -0.12 * k)
        return p

    def make(self, arm):
        bake(arm, "idle", 90, self.idle, loop=True)
        bake(arm, "walk", 36, self.hop, loop=True, stride=0.8)
        bake(arm, "hex", sum(HEX), self.hex, marks=dict(move_marks(HEX), hit=HEX[0]))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 2, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 4, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


# ------------------------------------------------------------------ the minion


class Minion:
    def base(self, f, n=90, k=1.0):
        p = Pose()
        p.turn("body", E(0, 0, 3 * k * math.sin(TAU * f / n)))
        p.turn("head", E(2 * k * math.sin(TAU * f / n * 2), 8 * k * math.sin(TAU * (f / n - 0.3)), 0))
        p.turn("hat", E(0, 0, -2 * k * math.sin(TAU * (f / n - 0.1))))
        p.turn("hat.tip", E(4 * k * math.sin(TAU * (f / n - 0.2)), 0, -7 * k * math.sin(TAU * (f / n - 0.25))))
        p.turn("hem", E(0, 4 * math.sin(TAU * f / n), 0))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn("arm." + side, E(4 * k * math.sin(TAU * (f / 45 + 0.3 * sg)), 0, 0))
            p.turn("hand." + side, E(6 * k * math.sin(TAU * (f / 30 + sg * 0.2)), 0, 0))
        return p

    def idle(self, f):
        return self.base(f)

    def hop(self, f):
        """It scurries in little hops, its hat's tip flopping after it, its hands swinging."""
        n = 24
        u = f / n
        up = abs(math.sin(math.pi * u))
        p = self.base(f, n, 0.6)
        p["loc"] = (0.0, 0.07 * up, 0.0)
        y = 1 - 0.1 * math.cos(TAU * u)
        p["scale"] = {"body": (1 / math.sqrt(y), y, 1 / math.sqrt(y))}
        p.turn("body", E(8, 0, 6 * math.sin(math.pi * u)))
        p.turn("hat.tip", E(-18 * math.sin(TAU * (u - 0.2)), 0, -10 * math.sin(math.pi * u)))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn("arm." + side, E(25 * sg * math.sin(TAU * u), 0, 0))
        return p

    def bite(self, f):
        """Little hand swipes: arms drawn back and up, leaning away; then lunging in, both hands
        clawing down across him; then back."""
        s, a, r = BITE
        wind = env(f, [(0, 0.0), (s, 1.0), (s + 2, 0.0)])
        swipe = env(f, [(s - 1, 0.0), (s + 3, 1.0), (s + a + 3, 1.0), (s + a + r, 0.0)])
        p = self.base(f, 48, 0.4)
        p["loc"] = (0.0, 0.03 * wind, 0.14 * swipe)
        p.turn("body", E(-12 * wind + 18 * swipe, 0, 0))
        p.turn("head", E(-8 * wind + 6 * swipe, 0, 0))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn("arm." + side, E(-110 * wind + 40 * swipe, 0, sg * (20 * wind - 25 * swipe)))
            p.turn("hand." + side, E(-20 * wind + 30 * swipe, 0, 0))
        p.turn("hat.tip", E(25 * wind - 30 * swipe, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 8, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = self.base(f, 20, 1 + k)
        hop = abs(math.sin(TAU * f / 24)) * k
        p["loc"] = (0.0, 0.08 * hop, 0.0)
        for side, sg in (("L", 1), ("R", -1)):
            p.turn("arm." + side, E(-150 * k, 0, sg * 25 * math.sin(TAU * f / 12) * k))
        p.turn("head", E(-15 * k, 0, 0))
        p.turn("hat.tip", E(-20 * hop, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.base(f, 24, 1)
        p.turn("body", E(-18 * k, 0, 6 * k))
        p.turn("head", E(-15 * k, 0, 0))
        p.turn("hat.tip", E(30 * k, 0, 0))
        for side in ("L", "R"):
            p.turn("arm." + side, E(-50 * k, 0, 0))
        p["loc"] = (0.0, 0.0, -0.06 * k)
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        sway = math.sin(TAU * f / 20)
        p = self.base(f, 20, 1)
        p.turn("body", E(10 * k, 0, 15 * sway * k))
        p.turn("head", E(15 * k, 20 * sway * k, 0))
        p.turn("hat", E(0, 0, -20 * sway * k))
        p.turn("hat.tip", E(0, 0, -30 * sway * k))
        for side, sg in (("L", 1), ("R", -1)):
            p.turn("arm." + side, E(-30 * k, 0, sg * 40 * k))
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.base(f, 20, 1)
        p.turn("body", E(-22 * k, 0, 0))
        p.turn("hat.tip", E(35 * k, 0, 0))
        for side in ("L", "R"):
            p.turn("arm." + side, E(-80 * k, 0, 0))
        p["loc"] = (0.0, 0.0, -0.1 * k)
        return p

    def make(self, arm):
        bake(arm, "idle", 90, self.idle, loop=True)
        bake(arm, "walk", 24, self.hop, loop=True, stride=0.45)
        bake(arm, "bite", sum(BITE), self.bite, marks=move_marks(BITE))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


# ------------------------------------------------------------------ the jorogumo's legs

# Each leg's reach from her carapace (level, unit: `anim::foe_beasts::JORO_LEGS`); legs 0, 2, 4, 6 on
# her right (the sculpt's low x), front to back.
JORO_DIRS = [(-0.4695, 0.8829), (0.4695, 0.8829), (-0.8829, 0.4695), (0.8829, 0.4695),
             (-0.9511, -0.309), (0.9511, -0.309), (-0.5299, -0.848), (0.5299, -0.848)]
# The alternating tetrapod: her left front, right second, left third and right hind together, then
# the other four.
TETRAPOD = [1, 0, 0, 1, 1, 0, 0, 1]


class Jorogumo:
    @staticmethod
    def leg(p, i, swing, lift, knee=0.0):
        """Leg `i` swung `swing` degrees back along her body (whatever way it reaches), lifted `lift`
        degrees at its root, its knee bent `knee` degrees more (its point drawn in)."""
        dx, dz = JORO_DIRS[i]
        k = Vector((dz, 0.0, -dx))
        side = 1.0 if dx > 0 else -1.0
        p.turn(f"leg{i}.a", A(k, -lift) @ A(Y, side * swing))
        p.turn(f"leg{i}.b", A(k, -knee))
        p.turn(f"leg{i}.c", A(k, knee * 0.4))

    def upper(self, p, f, n=120, k=1.0):
        """Her upper body, poised (its own clips are Mixamo's, laid over these)."""
        p.turn("hips", E(0, 6 * k * math.sin(TAU * f / n), 0))
        p.turn("chest", E(2 * k * math.sin(TAU * f / 60), 0, 0))
        p.turn("head", E(3 * k * math.sin(TAU * f / 80), 10 * k * math.sin(TAU * (f / n - 0.2)), 0))
        p.turn("upper_arm.L", E(4 * k * math.sin(TAU * f / 70), 0, 0))
        p.turn("upper_arm.R", E(4 * k * math.sin(TAU * (f / 70 + 0.4)), 0, 0))
        return p

    def idle(self, f):
        n = 120
        p = self.upper(Pose(), f, n)
        p["loc"] = (0.0, 0.006 * math.sin(TAU * f / 60), 0.0)
        p.turn("abdomen", E(3 * math.sin(TAU * f / 60), 4 * math.sin(TAU * f / n), 0))
        for i in range(8):
            # (A leg now and then shifts its footing.)
            shift = max(0.0, math.sin(TAU * (f / n - i * 0.125))) ** 6
            self.leg(p, i, 2 * math.sin(TAU * (f / n + i * 0.3)), 10 * shift, 6 * shift)
        return p

    def walk(self, f):
        n = 40
        p = self.upper(Pose(), f, n, 0.5)
        p["loc"] = (0.0, 0.01 * math.cos(TAU * 2 * f / n), 0.0)
        p.turn("carapace", E(0, 0, 2 * math.sin(TAU * f / n)))
        p.turn("abdomen", E(4 * math.sin(TAU * 2 * f / n), 6 * math.sin(TAU * f / n), 0))
        for i in range(8):
            ph = (f / n + 0.5 * TETRAPOD[i]) % 1.0
            if ph < 0.5:
                swing, lift = -14 + 28 * ph / 0.5, 0.0
            else:
                u = (ph - 0.5) / 0.5
                swing, lift = 14 - 28 * smooth(u), 20 * math.sin(math.pi * u)
            self.leg(p, i, swing, lift, lift * 0.5)
        return p

    def braced(self, p, k):
        """Her legs set wide and low, braced to cast."""
        p["loc"] = (0.0, -0.03 * k, 0.0)
        for i in range(8):
            self.leg(p, i, 0.0, -6 * k, -8 * k)
        return p

    def hex(self, f):
        s, a, r = HEX
        n = s + a + r
        k = env(f, [(0, 0.0), (s, 1.0), (s + 10, 1.0), (n, 0.0)])
        throw = env(f, [(s - 2, 0.0), (s + 3, 1.0), (s + 14, 0.6), (n, 0.0)])
        p = self.braced(Pose(), k)
        p.turn("chest", E(-10 * k + 18 * throw, 15 * k - 25 * throw, 0))
        p.turn("upper_arm.L", E(-160 * k + 90 * throw, 0, 20 * k))
        p.turn("forearm.L", E(-20 * k + 20 * throw, 0, 0))
        p.turn("upper_arm.R", E(-40 * k, 0, -20 * k))
        p.turn("head", E(-6 * k + 8 * throw, 0, 0))
        p.turn("abdomen", E(-8 * k, 0, 0))
        return p

    def rally(self, f):
        s, a, r = RALLY
        n = s + a + r
        k = env(f, [(0, 0.0), (s - 10, 1.0), (n - 10, 1.0), (n, 0.0)])
        p = Pose()
        p["loc"] = (0.0, 0.05 * k, 0.0)
        p.turn("carapace", E(-12 * k, 0, 0))
        for i in range(8):
            front = i < 2
            self.leg(p, i, -10 * k if front else 4 * k, (45 if front else 4) * k + (8 * math.sin(TAU * f / 10) * k if front else 0), (30 if front else 0) * k)
        p.turn("chest", E(-12 * k, 0, 0))
        p.turn("head", E(-15 * k, 0, 0))
        p.turn("upper_arm.L", E(-120 * k, 0, 30 * k))
        p.turn("upper_arm.R", E(-120 * k, 0, -30 * k))
        p.turn("abdomen", E(-15 * k, 0, 0))
        return p

    def hit(self, f, n=24):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.upper(Pose(), f)
        p.turn("chest", E(-18 * k, 0, 8 * k))
        p.turn("head", E(-12 * k, 0, 0))
        p.turn("carapace", E(-6 * k, 0, 0))
        for i in range(8):
            self.leg(p, i, 0.0, 6 * k, 4 * k)
        return p

    def stagger(self, f, n=48):
        k = env(f, [(0, 0.0), (5, 1.0), (n - 12, 0.8), (n, 0.0)])
        sway = math.sin(TAU * f / 22)
        p = self.upper(Pose(), f)
        p["loc"] = (0.0, -0.04 * k, 0.0)
        p.turn("carapace", E(6 * k, 0, 10 * sway * k))
        p.turn("chest", E(15 * k, 0, -12 * sway * k))
        for i in range(8):
            self.leg(p, i, 6 * sway * k, 12 * k * (0.5 + 0.5 * math.sin(TAU * f / 9 + i)), 10 * k)
        return p

    def deflected(self, f, n=20):
        k = env(f, [(0, 0.0), (3, 1.0), (n, 0.0)])
        p = self.upper(Pose(), f)
        p.turn("chest", E(-25 * k, 0, 0))
        p.turn("carapace", E(-8 * k, 0, 0))
        p["loc"] = (0.0, 0.0, -0.1 * k)
        for i in range(2):
            self.leg(p, i, 0.0, 30 * k, 20 * k)
        return p

    def make(self, arm):
        bake(arm, "idle", 120, self.idle, loop=True)
        bake(arm, "walk", 40, self.walk, loop=True, stride=1.0)
        bake(arm, "hex", sum(HEX), self.hex, marks=dict(move_marks(HEX), hit=HEX[0]))
        bake(arm, "rally", sum(RALLY), self.rally, marks=move_marks(RALLY))
        bake(arm, "hit", 24, self.hit, marks={"impact": 3, "recover_end": 24})
        bake(arm, "stagger", 48, self.stagger, marks={"impact": 5, "recover_end": 48})
        bake(arm, "deflected", 20, self.deflected, marks={"recover_end": 20})


FOES = {
    # The raven: quick, light beats.
    "bat": Flyer(beat=24, amp=50, bob=0.03, stride=1.1),
    # Its night kin: bigger, heavier beats, and a cape of wings folded about it between them.
    "vampire": Flyer(beat=30, amp=55, bob=0.045, wrap=True, stride=1.3),
    # The large bat: slow beats, swerving banks.
    "large_bat": Flyer(beat=36, amp=44, bob=0.05, bank=16, stride=1.6),
    "werewolf": Wolf(trot=32, gallop=24, strides=(0.55, 1.3)),
    "dire_wolf": Wolf(trot=36, gallop=26, strides=(0.65, 1.5), heavy=1.25),
    "cobra": Cobra(),
    "slime": Slime(),
    "minion": Minion(),
    "jorogumo": Jorogumo(),
}


def main():
    a = vh.args()
    if len(a) < 3 or a[0] not in FOES:
        sys.exit(__doc__)
    foe, rig, out = a[0], a[1], a[2]
    vh.fresh_scene()
    arm = vh.import_rig(rig)
    arm.animation_data_create()
    FOES[foe].make(arm)
    bpy.context.scene.render.fps = vh.FPS
    first = next((n for n in ("idle", "hover") if n in bpy.data.actions), None)
    arm.animation_data.action = bpy.data.actions.get(first)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out))
    print("block_foe: saved", out)


if __name__ == "__main__":
    main()
