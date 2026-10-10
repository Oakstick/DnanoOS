"""The pilot's first clips, keyed by script on Ragnar's skeleton, so the whole chain can be shown before
any outside motion arrives: idle, run, the swing both ways, the block, the leap and a flinch.

    AdventuresOfTheAncientBlades --export-rig ragnar ragnar_rig.glb
    blender -b -P tools/blender/block_ragnar.py -- ragnar_rig.glb ragnar.blend

Each clip is an action on his armature with its markers (pose markers on the action, the
vocabulary of ANIMATION.md), `vh_loop` and, for a gait, `vh_stride` (metres a cycle carries him).
Keyed with the principles: anticipation before each strike and leap, the hips leading the chest
and the chest the arm, overlap in the head and the shield arm, arcs for the hands, and eases
everywhere except into a strike's impact, which snaps. `export_clips.py` then writes them out.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import vh_blender as vh  # noqa: E402
from posing import Pose, Rig, X, Y, Z, mix, v  # noqa: E402

# ------------------------------------------------------------------ the stance


def ready(rig, breathe=0.0, shift=0.0, look=0.0):
    """His guard, standing: the shield-side foot forward and the body turned a little behind the
    shield, knees soft, the shield before his left side and the blade up before his right
    shoulder. `breathe` (0 out, 1 in) lifts his chest; `shift` (-1 to 1) moves his weight onto his
    left (+) or right foot; `look` turns his head (degrees)."""
    j = rig.j
    p = Pose(rig)
    p.loc = v(0.006 * shift, -0.014 + 0.003 * breathe, 0.0)
    p.euler("hips", y=-10, z=2.0 * shift)
    p.euler("spine", x=5 + 1.0 * breathe, y=-2, z=-1.5 * shift)
    p.euler("chest", x=1 - 2.5 * breathe, y=-6)
    p.euler("neck", x=-3 + breathe)
    p.euler("head", x=-2, y=16 + look, z=-1.0 * shift)
    p.leg("L", j["foot.L"] + v(0.012, 0, 0.055), foot_yaw=-8)
    p.leg("R", j["foot.R"] + v(-0.02, 0, -0.05), foot_yaw=-22)
    # The shield before his left side, its face turned forward and out.
    p.arm("L", upper=(0.32, -0.88, 0.34 + 0.03 * breathe), fore=(0.05, -0.62, 0.78), fore_front=(0.55, 0.0, 0.84), fore_rest_front=X)
    # The blade up before his right shoulder, its edge forward.
    p.arm("R", upper=(-0.38, -0.86, 0.34 + 0.03 * breathe), fore=(0.12, -0.12, 0.98), upper_front=(0, 0, 1), fore_front=(0, 1, 0))
    p.weapon((-0.08, 0.93, 0.36), flat=(1, 0, 0))
    p.weapon((0.08, 0.93, 0.36), flat=(1, 0, 0), bone="offhand")
    return p


def idle(arm, rig):
    """Two seconds, looping: a breath in and out, his weight shifting from foot to foot and back,
    a glance about."""
    vh.new_action(arm, "idle")
    keys = [(0, 0.0, 0.0, 0.0), (30, 0.6, 1.0, 4.0), (60, 1.0, 0.0, 0.0), (90, 0.4, -1.0, -5.0), (120, 0.0, 0.0, 0.0)]
    for f, breathe, shift, look in keys:
        ready(rig, breathe, shift, look).key(arm, f)
    return {"loop": True}


def run(arm, rig):
    """A run, 26 frames a cycle (two strides): the heel strikes ahead of him, the hips sink lowest
    over the planted foot, the toe pushes off behind, the heel kicks up, the knee drives through
    high and the foot reaches for the next stride. Each ankle follows that path (an IK keeps the
    knee forward and the planted foot on the floor). The arms pump against the legs, the sword
    elbow bent, the shield carried before him; the hips turn with the striding leg and the chest
    against them; he leans into it, his head steady."""
    vh.new_action(arm, "run")
    j = rig.j
    n = 26
    # One ankle's path from its heel strike, relative to where it rests: (forward, up, the foot's
    # pitch in the model, toe down positive).
    path = [(0.10, 0.0, -14), (0.01, 0.0, 0), (-0.10, 0.012, 34), (-0.15, 0.085, 48), (-0.02, 0.12, 18), (0.12, 0.05, -8)]
    # The hips' height through each stride (it repeats every three keys): down as a foot lands,
    # lowest over it, up as it pushes off into the flight.

    def at(k):
        p = Pose(rig)
        share = k / 6
        swing = math.cos(2 * math.pi * share)  # 1 as his left heel strikes
        p.loc = v(0.0, [-0.024, -0.036, -0.012][k % 3], 0.0)
        p.euler("hips", y=-10 * swing, z=2.5 * math.sin(2 * math.pi * share))
        p.euler("spine", x=11, y=5 * swing)
        p.euler("chest", x=4, y=9 * swing)
        p.euler("neck", x=-4, y=-4 * swing)
        p.euler("head", x=-6, y=-6 * swing)
        for side, lag in (("L", 0), ("R", 3)):
            fwd, up, pitch = path[(k + lag) % 6]
            p.leg(side, j[f"foot.{side}"] + v(0.0, up, fwd), foot_pitch=pitch)
        # The sword arm pumps forward as his left foot strikes; the shield arm against the right.
        pump = swing
        p.arm("R", upper=(-0.3, -0.85 + 0.25 * max(pump, 0), 0.42 * pump), fore=(0.1, 0.2 + 0.45 * max(pump, 0), 0.95), fore_front=(0, 1, 0))
        p.weapon((-0.05, 0.85, -0.5 + 0.6 * max(pump, 0)), flat=(1, 0, 0))
        p.weapon((0.05, 0.85, -0.5), flat=(1, 0, 0), bone="offhand")
        p.arm("L", upper=(0.36, -0.86, 0.36 - 0.25 * pump), fore=(0.02, -0.2, 0.98), fore_front=(0.6, 0, 0.8), fore_rest_front=X)
        return p

    for k in range(6):
        at(k).key(arm, round(k * n / 6))
    at(0).key(arm, n)
    return {"loop": True, "stride": 2.2, "markers": {"foot_l": 0, "foot_r": n // 2}}


# ------------------------------------------------------------------ strikes


def strike_pose(rig, base, hips_y, chest_y, lean, step, upper, fore, blade, flat=(0, 1, 0), shield=None):
    """A moment of a swing: the torso turned (hips leading, the chest following: `chest_y` is the
    chest's turn in the model), the weight stepped `step` metres forward onto the front foot, his
    eyes kept on the foe before him; the sword arm's upper arm and forearm along `upper` and
    `fore`, the blade along `blade` with its flat toward `flat`, all in the model's axes (the foe is
    at +Z), as is the shield arm `shield` (upper, forearm, the shield's face)."""
    j = rig.j
    p = base.copy()
    p.loc = v(0.0, -0.016 - abs(step) * 0.2, step)
    p.euler("hips", y=hips_y)
    p.euler("spine", x=5 + lean, y=(chest_y - hips_y) * 0.4)
    p.euler("chest", x=lean * 0.4, y=(chest_y - hips_y) * 0.6)
    p.euler("neck", x=-3, y=-chest_y * 0.4)
    p.euler("head", x=-lean * 0.7, y=-chest_y * 0.5 + 6)
    p.leg("L", j["foot.L"] + v(0.012, 0, 0.055), foot_yaw=-8)
    p.leg("R", j["foot.R"] + v(-0.02, 0, -0.05), foot_yaw=-22)
    p.arm("R", upper=upper, fore=fore, fore_front=(0, 1, 0), frame=None)
    p.weapon(blade, flat, frame=None)
    if shield:
        p.arm("L", upper=shield[0], fore=shield[1], fore_front=shield[2], fore_rest_front=X, frame=None)
    return p


def slash(arm, rig, side):
    """A swing, 40 frames as keyed (the game warps it onto the move's frames): from the guard, the
    wind-up (the blade drawn back over the shoulder, hips and chest coiled away, the weight on the
    back foot), the strike (the hips turn first, then the chest, the elbow leads and the blade
    lags, then whips through), the follow-through past the target, and the settle back to guard.
    `side` 1 is the forehand (from his right across to his left), -1 the backhand."""
    name = "slash_fore" if side > 0 else "slash_back"
    vh.new_action(arm, name)
    base = ready(rig)
    guard = ((0.3, -0.85, 0.42), (0.05, -0.55, 0.83), (0.6, 0.0, 0.8))
    tuck = ((0.3, -0.75, 0.6), (-0.3, -0.35, 0.88), (0.35, 0.15, 0.92))
    S = strike_pose
    if side > 0:
        # A diagonal cut: from high over his right shoulder, down through the foe, to low at his left.
        windup = S(rig, base, -18, -35, -3, -0.015, (-0.55, 0.75, -0.35), (-0.15, 0.85, -0.5), (0.2, 0.6, -0.78), shield=tuck)
        launch = S(rig, base, 6, -18, 2, 0.0, (-0.75, 0.55, 0.35), (-0.3, 0.6, 0.75), (-0.2, 0.75, -0.6), shield=tuck)
        hit = S(rig, base, 18, 5, 6, 0.04, (-0.4, 0.2, 0.9), (0.2, -0.2, 0.96), (0.5, -0.25, 0.83), shield=guard)
        through = S(rig, base, 26, 24, 8, 0.045, (0.25, -0.2, 0.95), (0.75, -0.45, 0.48), (0.8, -0.55, -0.2), shield=guard)
        follow = S(rig, base, 28, 36, 7, 0.04, (0.45, -0.55, 0.7), (0.55, -0.75, -0.35), (0.1, -0.8, -0.6), shield=guard)
    else:
        # A flat cut back: from over his left shoulder, across at the height of his shoulders, out to his right.
        windup = S(rig, base, 12, 30, -3, -0.015, (0.55, 0.15, 0.82), (0.5, 0.6, -0.62), (-0.5, 0.5, -0.7), shield=guard)
        launch = S(rig, base, -4, 18, 2, 0.0, (0.3, 0.2, 0.93), (0.05, 0.4, 0.92), (0.6, 0.45, -0.66), shield=guard)
        hit = S(rig, base, -18, -5, 5, 0.04, (-0.2, 0.1, 0.97), (-0.55, 0.0, 0.83), (-0.6, 0.1, 0.8), shield=tuck)
        through = S(rig, base, -26, -24, 6, 0.045, (-0.7, 0.05, 0.71), (-0.97, 0.0, 0.25), (-0.85, 0.05, -0.5), shield=tuck)
        follow = S(rig, base, -28, -36, 5, 0.04, (-0.9, -0.1, 0.4), (-0.75, -0.25, -0.6), (-0.2, -0.2, -0.95), shield=tuck)
    # Frames: the guard, the wind-up held a breath, the launch, the impact, the blade through,
    # the follow-through's end, the settle.
    base.key(arm, 0)
    windup.key(arm, 7)
    mix(windup, launch, 0.35).key(arm, 9)  # (the wind-up's last frames: already turning into the strike)
    launch.key(arm, 10, "LINEAR")
    hit.key(arm, 12, "SINE", "EASE_OUT")
    through.key(arm, 15)
    follow.key(arm, 22)
    mix(follow, base, 0.6).key(arm, 30)
    base.key(arm, 40)
    # (The wind-up ends as the blade reaches the foe: the move's active frames are its blow.)
    return {"markers": {"windup_end": 12, "hit": 12, "active_end": 15, "recover_end": 40}}


# ------------------------------------------------------------------ defence


def block(arm, rig):
    """The guard: the shield snapped up before him (6 frames as keyed; the game fits them into the
    guard's two), the forearm across his chest, its face forward, his head ducked behind its rim,
    the sword drawn back ready; then held, breathing, for a second that loops."""
    vh.new_action(arm, "block")
    j = rig.j

    def up(breathe):
        p = ready(rig, breathe)
        p.loc = v(0.0, -0.024 + 0.002 * breathe, 0.0)
        # Turned behind the shield, his left shoulder leading, his eyes on the foe over its rim.
        p.euler("hips", y=-20)
        p.euler("spine", x=9 + breathe, y=-6)
        p.euler("chest", x=4 - breathe * 1.5, y=-12)
        p.euler("neck", y=14)
        p.euler("head", x=8, y=22)
        p.leg("L", j["foot.L"] + v(0.015, 0, 0.065), foot_yaw=-8)
        p.leg("R", j["foot.R"] + v(-0.025, 0, -0.06), foot_yaw=-22)
        # The shield forearm across his front, the shield's face square to the foe (in the model's axes).
        p.arm("L", upper=(-0.3, -0.38, 0.88), fore=(-0.96, 0.12, 0.12), fore_front=(0.0, 0.0, 1.0), fore_rest_front=X, frame=None)
        p.arm("R", upper=(-0.5, -0.55, -0.1), fore=(-0.05, 0.25, 0.97), fore_front=(0, 1, 0))
        p.weapon((-0.25, 0.6, 0.76), flat=(1, 0, 0))
        return p

    ready(rig).key(arm, 0)
    over = up(0.0)
    over.euler("chest", x=6, y=-2)
    over.key(arm, 4, "SINE", "EASE_OUT")
    up(0.0).key(arm, 6)
    up(1.0).key(arm, 36)
    up(0.0).key(arm, 66)
    return {"markers": {"raised": 6}}


def jump(arm, rig):
    """The leap (25 frames in the air as keyed, the game's leap stretched onto them): he springs off
    his toes, arms swinging up, tucks his knees at the top, the blade held high and the shield
    before him, reaches down for the floor, lands into a deep crouch (the landing, 12 frames more)
    and rises back into his guard."""
    vh.new_action(arm, "jump")
    j = rig.j
    base = ready(rig)

    def air(lift, thighs, shins, feet, arms_up, lean, tuck_l=0.0):
        p = base.copy()
        p.loc = v(0.0, lift, 0.0)
        p.euler("hips", y=-6, x=-4 + lean * 0.3)
        p.euler("spine", x=lean)
        p.euler("chest", x=lean * 0.4, y=-4)
        p.euler("head", x=-lean * 0.8, y=10)
        p.leg_fk("L", thighs[0], shins[0], 0, roll=2)
        p.leg_fk("R", thighs[1], shins[1], 0, roll=-2)
        for side, th, sh in (("L", thighs[0], shins[0]), ("R", thighs[1], shins[1])):
            p.q[f"foot.{side}"] = vh.quat_g(feet - th - sh)
        p.arm("R", upper=(-0.45, -0.5 + arms_up, 0.4 - 0.3 * arms_up), fore=(0.0, 0.5 + 0.4 * arms_up, 0.8 - 0.5 * arms_up), fore_front=(0, 1, 0))
        p.weapon((-0.1, 0.75, -0.65), flat=(1, 0, 0))
        p.arm("L", upper=(0.4, -0.5 + 0.5 * arms_up, 0.6), fore=(-0.2, 0.1 + 0.3 * arms_up, 0.95), fore_front=(0.4, 0.0, 0.9), fore_rest_front=X)
        return p

    def crouch(depth, lean):
        p = base.copy()
        p.loc = v(0.0, -depth, 0.01)
        p.euler("hips", y=-8, x=6)
        p.euler("spine", x=8 + lean)
        p.euler("chest", x=lean * 0.5, y=-6)
        p.euler("head", x=-lean * 0.7 - 4, y=14)
        p.leg("L", j["foot.L"] + v(0.03, 0, 0.06), foot_yaw=-8)
        p.leg("R", j["foot.R"] + v(-0.035, 0, -0.04), foot_yaw=-22)
        p.arm("R", upper=(-0.55, -0.75, 0.3), fore=(0.1, -0.1, 0.99), fore_front=(0, 1, 0))
        p.weapon((-0.15, 0.85, 0.5), flat=(1, 0, 0))
        p.arm("L", upper=(0.45, -0.8, 0.4), fore=(0.15, -0.5, 0.85), fore_front=(0.6, 0, 0.8), fore_rest_front=X)
        return p

    air(0.004, (12, 18), (6, 8), 38, 0.55, -4).key(arm, 0)  # off his toes
    air(0.02, (-55, -42), (82, 70), 18, 1.0, 6).key(arm, 6)
    air(0.03, (-82, -74), (122, 112), 6, 1.0, 14).key(arm, 12)  # the tuck, at the top
    air(0.012, (-40, -34), (42, 38), -14, 0.6, 8).key(arm, 20)  # reaching down
    air(0.0, (-26, -20), (30, 26), -6, 0.4, 6).key(arm, 25)  # touching down
    crouch(0.06, 16).key(arm, 29, "SINE", "EASE_OUT")
    crouch(0.035, 10).key(arm, 33)
    base.key(arm, 37)
    return {"markers": {"takeoff": 0, "apex": 12, "land": 25, "recover_end": 37}}


def hit(arm, rig):
    """A blow taken (laid over his upper body; his legs walk on): his chest and head thrown back
    and turned from it, the arms flung out, then gathered back to guard."""
    vh.new_action(arm, "hit")
    base = ready(rig)
    struck = base.copy()
    struck.euler("spine", x=-10, y=10, z=4)
    struck.euler("chest", x=-12, y=8)
    struck.euler("neck", x=-8)
    struck.euler("head", x=-16, y=26, z=6)
    struck.arm("R", upper=(-0.6, -0.55, -0.2), fore=(-0.2, 0.3, 0.9), fore_front=(0, 1, 0))
    struck.weapon((-0.4, 0.8, -0.4), flat=(1, 0, 0))
    struck.arm("L", upper=(0.6, -0.6, 0.2), fore=(0.3, -0.4, 0.86), fore_front=(0.8, 0, 0.6), fore_rest_front=X)
    base.key(arm, 0)
    struck.key(arm, 3, "SINE", "EASE_OUT")
    mix(struck, base, 0.45).key(arm, 10)
    base.key(arm, 24)
    return {"markers": {"impact": 3, "recover_end": 24}}


CLIPS = [idle, run, lambda a, r: slash(a, r, 1), lambda a, r: slash(a, r, -1), block, jump, hit]


def main():
    a = vh.args()
    if len(a) < 2:
        sys.exit(__doc__)
    vh.fresh_scene()
    arm = vh.import_rig(a[0])
    rig = Rig(arm)
    for make in CLIPS:
        info = make(arm, rig)
        act = arm.animation_data.action
        keys = sorted({round(kp.co.x) for fc in vh.fcurves(act) for kp in fc.keyframe_points})
        act.frame_range = (keys[0], keys[-1])
        act.use_frame_range = True
        act["vh_loop"] = bool(info.get("loop", False))
        if "stride" in info:
            act["vh_stride"] = float(info["stride"])
        for name, frame in info.get("markers", {}).items():
            m = act.pose_markers.new(name)
            m.frame = int(frame)
        print(f"block_ragnar: {act.name}: frames {keys[0]}-{keys[-1]}, markers {dict(info.get('markers', {}))}")
    arm.animation_data.action = bpy.data.actions["idle"]
    bpy.context.scene.frame_start, bpy.context.scene.frame_end = 0, 120
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(a[1]))
    print("block_ragnar: saved", a[1])


main()
