"""Ragnar's clips from Mixamo's Pro Sword And Shield Pack (and two extras): the picks, each retargeted
onto his armature with the corrections his build needs, its markers found from the motion itself,
written as actions into his .blend (replacing the scripted ones of the same name).

    blender -b ragnar.blend -P tools/blender/mixamo_ragnar.py -- MIXAMO_DIR OUT.blend [slot ...]

MIXAMO_DIR holds `sword-shield/` (the pack's FBX files) and `extra/` ("Dodging Right", "Stand To
Roll"). The FBX files stay outside the repository (Mixamo's terms: its motion may ship in the game,
its files may not be passed on); only the clips exported from OUT.blend go into `assets/anim/`.

Why each pick, and what was corrected, is in ANIMATION.md §6. The markers:

* a strike's from its blade: the tip's speed is measured every frame, `hit` at its fastest,
  `windup_end` where it first runs at half that (the blade on its way to the foe), `active_end`
  where it falls back under half, `recover_end` at the end; the lead-in before the blade first
  stirs is cut away;
* the leap's from the feet: `takeoff` the first frame both are off the floor, `land` the first
  one back, `apex` the hips' top;
* the guard's raise from the shield: `raised` where it stops rising;
* the flinch's `impact` at the chest's quickest turn.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import retarget as rt  # noqa: E402
import vh_blender as vh  # noqa: E402
from mathutils import Vector  # noqa: E402

SS = "sword-shield/sword and shield "

# How his blade sits in his fist (degrees about the hand's x from along the forearm, `retarget.HOLD`):
# square to the forearm, out of the thumb's side, as Mixamo's fist holds it.
HOLD = 90.0

# slot: (file, options). `turn` and `yaw_keep` as in retarget.Retarget; `kind` picks the markers.
# His fighting stance faces the foe side-on, shield first: his hips 50 degrees to his right (Mixamo's
# own is 55 to 67). Every clip of the stance is set to it (`face`), so the guard laid over the idle
# or the walk sits on the hips it was made on, and a strike starts from the stance it leaves.
STANCE = -50.0

PICKS = {
    # A steady guard, shield forward, breathing.
    "idle": (SS + "idle (4).fbx", dict(face=STANCE, loop=True, arms_out=6)),
    # The walk under the guard (he walks at about half his pace behind his shield), side-on as well.
    "walk": (SS + "walk.fbx", dict(face=STANCE + 15, loop=True, stride=1.3, arms_out=6)),
    "run": (SS + "run.fbx", dict(loop=True, stride=2.4, arms_out=8)),
    # The forehand: high from his right, down across to his left; its sword arm swung 35 degrees
    # higher, so the cut passes above his shoulders where the camera behind him sees it (the blade
    # in view on 69% of its active frames from there; the scripted forehand's, 10%).
    "slash_fore": (SS + "slash (5).fbx", dict(kind="strike", face=STANCE + 10, arms_out=6, sword_turn=(0, 35))),
    # The backhand: the same cut run backward, rising from low at his left out to high at his right.
    # Mixamo's own backhands spin him round (a whole turn in a fifth of a second, warped): this one
    # chains with the forehand (one's end is the other's start) and reads as well from behind.
    "slash_back": (SS + "slash (5).fbx", dict(kind="strike", reverse=True, face=STANCE + 10, arms_out=6, sword_turn=(0, 35))),
    # The finisher (L3): the spinning cut, spin and all, the arm raised.
    "slash_heavy": (SS + "slash (3).fbx", dict(kind="strike", face=STANCE, arms_out=6, sword_turn=(15, 30))),
    # The guard: the shield snapped up square before him (its centre on his middle, its face to the foe).
    "block": (SS + "block.fbx", dict(kind="raise", face=STANCE)),
    "block_idle": (SS + "block idle.fbx", dict(loop=True, face=STANCE)),
    "jump": (SS + "jump.fbx", dict(kind="leap", arms_out=6)),
    # The evade: a dive and roll, its turning taken out (the game steers him).
    "roll": ("extra/Stand To Roll.fbx", dict(kind="leap", yaw_keep=0.0, face=0.0)),
    "hit": (SS + "impact (2).fbx", dict(kind="flinch", face=STANCE)),
}


def tip_track(arm, act):
    """The blade's tip (game axes) every frame of `act`, and the feet's and hips' heights."""
    scene = bpy.context.scene
    arm.animation_data.action = act
    f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
    tips, feet, hips, shield, chest = [], [], [], [], []
    for f in range(f0, f1 + 1):
        scene.frame_set(f)
        m = arm.matrix_world @ arm.pose.bones["blade"].matrix
        tips.append(vh.b2g(m @ Vector((0, 0.3, 0))))
        feet.append(min(vh.b2g(arm.matrix_world @ arm.pose.bones[b].head).y for b in ("foot.L", "foot.R")))
        hips.append(vh.b2g(arm.matrix_world @ arm.pose.bones["hips"].head).y)
        shield.append(vh.b2g(arm.matrix_world @ arm.pose.bones["shield"].head))
        chest.append((arm.matrix_world @ arm.pose.bones["chest"].matrix).to_quaternion())
    return tips, feet, hips, shield, chest


def strike_marks(tips):
    sp = [(tips[i + 1] - tips[i]).length * vh.FPS for i in range(len(tips) - 1)]
    k = max(range(len(sp)), key=lambda i: sp[i])
    half = sp[k] * 0.5
    a = k
    while a > 0 and sp[a - 1] >= half:
        a -= 1
    b = k
    while b < len(sp) - 1 and sp[b + 1] >= half:
        b += 1
    stir = next((i for i in range(len(sp)) if sp[i] > sp[k] * 0.12), 0)
    return {"lead": max(0, stir - 3), "windup_end": a, "hit": k, "active_end": b + 1}


def leap_marks(feet, hips):
    ground = min(feet[0], feet[-1])
    up = [i for i, y in enumerate(feet) if y > ground + 0.02]
    if not up:
        # (A roll stays on the floor: its dive is where his hips sink, its rise where they come back.)
        low = [i for i, y in enumerate(hips) if y < hips[0] - 0.06]
        return {"takeoff": low[0], "apex": min(range(len(hips)), key=lambda i: hips[i]), "land": low[-1]}
    takeoff = up[0]
    land = next((i for i in range(takeoff, len(feet)) if feet[i] <= ground + 0.02), len(feet) - 1)
    apex = max(range(takeoff, land + 1), key=lambda i: hips[i])
    return {"takeoff": takeoff, "apex": apex, "land": land}


def raise_marks(shield):
    sp = [(shield[i + 1] - shield[i]).length for i in range(len(shield) - 1)]
    k = max(range(len(sp)), key=lambda i: sp[i])
    r = next((i for i in range(k, len(sp)) if sp[i] < sp[k] * 0.2), len(sp) - 1)
    return {"raised": r + 1}


def flinch_marks(chest):
    sp = [chest[i].rotation_difference(chest[i + 1]).angle for i in range(len(chest) - 1)]
    return {"impact": max(range(len(sp)), key=lambda i: sp[i]) + 1}


def trim(act, start):
    """Cuts the first `start` frames off `act` (keys and markers move down with it)."""
    if start <= 0:
        return
    for fc in vh.fcurves(act):
        for kp in list(fc.keyframe_points):
            if kp.co.x < start - 1e-3:
                fc.keyframe_points.remove(kp)
        for kp in fc.keyframe_points:
            kp.co.x -= start
            kp.handle_left.x -= start
            kp.handle_right.x -= start
    f0, f1 = act.frame_range
    act.frame_range = (0, f1 - start)


def reverse(act):
    """Plays `act` backward (a forehand run back is a backhand: the cut rising from his left out
    to his right)."""
    n = act.frame_range[1]
    for fc in vh.fcurves(act):
        pts = [(n - kp.co.x, kp.co.y) for kp in fc.keyframe_points]
        for kp, (x, y) in zip(fc.keyframe_points, pts):
            kp.co.x, kp.co.y = x, y
            kp.handle_left.x, kp.handle_left.y = x, y
            kp.handle_right.x, kp.handle_right.y = x, y
        fc.keyframe_points.sort() if hasattr(fc.keyframe_points, "sort") else None
        fc.update()


def make(slot, path, opts, tgt):
    old = bpy.data.actions.get(slot)
    if old:
        bpy.data.actions.remove(old)
    src, fps = rt.load_source(path)
    r = rt.Retarget(src, tgt, arms_out=opts.get("arms_out", 0.0), turn=opts.get("turn", 0.0), yaw_keep=opts.get("yaw_keep", 1.0), hold={"blade": opts.get("hold", HOLD)}, sword_turn=opts.get("sword_turn", (0.0, 0.0)), face=opts.get("face"))
    act = rt.bake(r, slot, loop=opts.get("loop", False), stride=opts.get("stride"), fps_src=fps)
    # (The source and its own action go: only his is kept.)
    for o in [o for o in bpy.data.objects if o.parent == src] + [src]:
        bpy.data.objects.remove(o)
    for leftover in {x.name for x in bpy.data.actions if x.name.startswith("Armature|")}:
        bpy.data.actions.remove(bpy.data.actions[leftover])
    if opts.get("reverse"):
        reverse(act)
    kind = opts.get("kind")
    marks = {}
    if kind:
        tips, feet, hips, shield, chest = tip_track(tgt, act)
        if kind == "strike":
            marks = strike_marks(tips)
            lead = marks.pop("lead")
            trim(act, lead)
            marks = {k: v - lead for k, v in marks.items()}
            marks["recover_end"] = int(act.frame_range[1])
        elif kind == "leap":
            marks = leap_marks(feet, hips)
            marks["recover_end"] = int(act.frame_range[1])
        elif kind == "raise":
            marks = raise_marks(shield)
        elif kind == "flinch":
            marks = flinch_marks(chest)
            marks["recover_end"] = int(act.frame_range[1])
    for k, v in marks.items():
        act.pose_markers.new(k).frame = int(v)
    act["vh_source"] = os.path.basename(path)
    print(f"mixamo_ragnar: {slot:12} from {os.path.basename(path):34} {int(act.frame_range[1]):4} frames, markers {marks}")
    return act


def parry(tgt):
    """The parry: Mixamo has none. The shield snapped up (the block's raised pose, reached in 3
    frames, against the block's 10), then the shield's recoil from the blow it turned (the hit
    flinch on his upper body, half as deep), and back."""
    old = bpy.data.actions.get("parry")
    if old:
        bpy.data.actions.remove(old)
    block, hit = bpy.data.actions["block"], bpy.data.actions["hit"]
    raised = int(block.pose_markers["raised"].frame)
    impact = int(hit.pose_markers["impact"].frame)
    scene = bpy.context.scene
    poses = []

    def grab(act, f):
        tgt.animation_data.action = act
        scene.frame_set(f)
        return {b: (pb.rotation_quaternion.copy(), pb.location.copy()) for b, pb in ((b, tgt.pose.bones[b]) for b in vh.BONES)}

    up = grab(block, raised)
    rest = grab(bpy.data.actions["block_idle"], 0)
    struck = grab(hit, impact)
    # (frame, pose, how far toward the flinch on the upper body)
    keys = [(0, rest, 0.0), (3, up, 0.0), (6, up, 0.5), (10, up, 0.25), (22, rest, 0.0)]
    act = vh.new_action(tgt, "parry")
    for f, pose, k in keys:
        for b in vh.BONES:
            pb = tgt.pose.bones[b]
            q, loc = pose[b]
            if k and b not in ("hips", "thigh.L", "shin.L", "foot.L", "thigh.R", "shin.R", "foot.R"):
                q = q.slerp(struck[b][0], k)
            pb.rotation_mode = "QUATERNION"
            pb.rotation_quaternion = q
            pb.keyframe_insert("rotation_quaternion", frame=f)
            if b == "hips":
                pb.location = loc
                pb.keyframe_insert("location", frame=f)
    act.frame_range = (0, 22)
    act.use_frame_range = True
    for name, f in (("raised", 3), ("impact", 6), ("recover_end", 22)):
        act.pose_markers.new(name).frame = f
    print("mixamo_ragnar: parry        from the block and the flinch, 22 frames")


def main():
    a = vh.args()
    if len(a) < 2:
        sys.exit(__doc__)
    root, out, only = a[0], a[1], set(a[2:])
    tgt = vh.armature()
    if tgt.animation_data is None:
        tgt.animation_data_create()
    for slot, (path, opts) in PICKS.items():
        if only and slot not in only:
            continue
        make(slot, os.path.join(root, path), opts, tgt)
    if not only or "parry" in only:
        parry(tgt)
    bpy.context.scene.render.fps = vh.FPS
    tgt.animation_data.action = bpy.data.actions.get("idle")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out))
    print("mixamo_ragnar: saved", out)


main()
