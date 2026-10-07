"""A hero's clips from Mixamo: the picks for each of the three (a table per hero), each retargeted onto
his armature with the corrections his build needs, its markers found from the motion itself, written
as actions into his .blend (replacing those of the same name).

    blender -b HERO.blend -P tools/blender/mixamo_hero.py -- HERO MIXAMO_DIR OUT.blend [slot ...]
    blender -b HERO.blend -P tools/blender/mixamo_hero.py -- HERO MIXAMO_DIR OUT.blend --try FILE[:OPTS] ...
    blender -b HERO.blend -P tools/blender/mixamo_hero.py -- HERO --measure [action ...]

HERO is ragnar, mehmed or miyamoto. MIXAMO_DIR holds the packs as downloaded: `sword-shield/` (the Pro
Sword And Shield Pack), `great-sword/` (the Great Sword Pack) and `extra/` ("Dodging Right", "Stand To
Roll"). The FBX files stay outside the repository (Mixamo's terms: its motion may ship in the game,
its files may not be passed on); only the clips exported from OUT.blend go into `assets/anim/`.

`--try` retargets candidates instead of the picks, each as an action named after its file (`FILE` is
relative to MIXAMO_DIR; `:OPTS` is a pick's options as Python, `face=-30,loop=True`), to look at
with `preview.py` before choosing. `--measure` reports, for each strike, the share of its active
frames on which the blade can be seen from the game's chase camera (behind and above him,
`camera::chase`), by casting rays from there against his voxels (the .blend must hold them, as
`import_rig.py` saves it).

Why each pick, and what was corrected, is in ANIMATION.md §6 (Ragnar), §6.5 (Mehmed) and §6.6
(Miyamoto). The markers:

* a strike's from its blade: the tip's speed is measured every frame, `hit` at its fastest,
  `windup_end` where it first runs at half that (the blade on its way to the foe), `active_end`
  where it falls back under half, `recover_end` at the end; the lead-in before the blade first
  stirs is cut away;
* the leap's from the feet: `takeoff` the first frame both are off the floor, `land` the first
  one back, `apex` the hips' top;
* the guard's raise from the shield (or, where the shield is slung, the off hand): `raised` where
  it stops rising;
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
GS = "great-sword/great sword "
ROLL = "extra/Stand To Roll.fbx"

# How a blade sits in the fist (degrees about the hand's x from along the forearm, `retarget.HOLD`)
# where the source has no fingers: square to the forearm, out of the thumb's side.
HOLD = 90.0

# ------------------------------------------------------------------ Ragnar

# His fighting stance faces the foe side-on, shield first: his hips 50 degrees to his right (Mixamo's
# own is 55 to 67). Every clip of the stance is set to it (`face`), so the guard laid over the idle
# or the walk sits on the hips it was made on, and a strike starts from the stance it leaves.
RAGNAR_STANCE = -50.0

# slot: (file, options). `turn`, `face`, `yaw_keep`, `arms_out`, `sword_turn` and `posture` as in
# retarget.Retarget; `still` as in retarget.bake; `kind` picks the markers; `reverse` plays the clip
# backward; `span` keeps only those seconds of the source; `cut` ends a strike at that frame.
RAGNAR = {
    # A steady guard, shield forward, breathing.
    "idle": (SS + "idle (4).fbx", dict(face=RAGNAR_STANCE, loop=True, arms_out=6)),
    # The walk under the guard (he walks at about half his pace behind his shield), side-on as well.
    "walk": (SS + "walk.fbx", dict(face=RAGNAR_STANCE + 15, loop=True, stride=1.3, arms_out=6)),
    "run": (SS + "run.fbx", dict(loop=True, stride=2.4, arms_out=8)),
    # The forehand: high from his right, down across to his left; its sword arm swung 35 degrees
    # higher, so the cut passes above his shoulders where the camera behind him sees it (the blade
    # in view on 69% of its active frames from there; the scripted forehand's, 10%).
    "slash_fore": (SS + "slash (5).fbx", dict(kind="strike", face=RAGNAR_STANCE + 10, arms_out=6, sword_turn=(0, 35))),
    # The backhand: the same cut run backward, rising from low at his left out to high at his right.
    # Mixamo's own backhands spin him round (a whole turn in a fifth of a second, warped): this one
    # chains with the forehand (one's end is the other's start) and reads as well from behind.
    "slash_back": (SS + "slash (5).fbx", dict(kind="strike", reverse=True, face=RAGNAR_STANCE + 10, arms_out=6, sword_turn=(0, 35))),
    # The finisher (L3): the spinning cut, spin and all, the arm raised.
    "slash_heavy": (SS + "slash (3).fbx", dict(kind="strike", face=RAGNAR_STANCE, arms_out=6, sword_turn=(15, 30))),
    # The guard: the shield snapped up square before him (its centre on his middle, its face to the foe).
    "block": (SS + "block.fbx", dict(kind="raise", face=RAGNAR_STANCE)),
    "block_idle": (SS + "block idle.fbx", dict(loop=True, face=RAGNAR_STANCE)),
    "jump": (SS + "jump.fbx", dict(kind="leap", arms_out=6)),
    # The evade: a dive and roll, its turning taken out (the game steers him).
    "roll": (ROLL, dict(kind="leap", yaw_keep=0.0, face=0.0)),
    "hit": (SS + "impact (2).fbx", dict(kind="flinch", face=RAGNAR_STANCE)),
}

# ------------------------------------------------------------------ Mehmed

# The janissary's carriage: squarer to the foe than Ragnar's side-on guard (his hips 30 degrees to
# his right, not 50), his back straight and his chest out (the spine and chest straightened 6 and 10
# degrees from Mixamo's crouch, the chest leaning a little back), his head kept as the source holds
# it; in the idle, where Mixamo's soldier looks down at his feet, his head raised to the foe. His
# wide sleeves carried clear of his kaftan.
MEHMED_STANCE = -30.0
MEHMED_POSTURE = {"spine": 6.0, "chest": 10.0, "head": -13.0}
MEHMED_HEAD_UP = {"spine": 6.0, "chest": 10.0, "neck": 10.0, "head": 17.0}

MEHMED = {
    # A calm three seconds of Idle (3): the yatagan turned idly in his fist, his weight settling.
    # (Idle (4) is Ragnar's; Idle (1) turns him round to look behind him.)
    "idle": (SS + "idle (3).fbx", dict(face=MEHMED_STANCE, loop=True, arms_out=8, posture=MEHMED_HEAD_UP, span=(4.0, 7.0))),
    "walk": (SS + "walk (2).fbx", dict(face=MEHMED_STANCE + 10, loop=True, stride=1.3, arms_out=8, posture=MEHMED_POSTURE)),
    # The guarded jog behind the shield: the soldier's run, kept in formation.
    "run": (SS + "run (2).fbx", dict(loop=True, stride=2.4, arms_out=8, posture={"spine": 4.0, "head": -4.0})),
    # The forehand: Slash (1), a diagonal cut from high at his right down across his front.
    "slash_fore": (SS + "slash.fbx", dict(kind="strike", face=MEHMED_STANCE, arms_out=8, posture=MEHMED_POSTURE, sword_turn=(15, 35))),
    # The backhand: Attack (3), the long low sweep from his left out to his right, its whirl in the
    # wind-up taken out (`yaw_keep` 0: he stays square, his chest coiling and uncoiling).
    "slash_back": (SS + "attack (3).fbx", dict(kind="strike", yaw_keep=0.0, face=MEHMED_STANCE, arms_out=8, posture=MEHMED_POSTURE, sword_turn=(0, 15))),
    # The finisher: Attack (1), the blade raised high over his head and brought down in a chop.
    "slash_heavy": (SS + "attack.fbx", dict(kind="strike", face=MEHMED_STANCE, arms_out=8, posture=MEHMED_POSTURE)),
    "block": (SS + "block (2).fbx", dict(kind="raise", face=MEHMED_STANCE, posture=MEHMED_POSTURE)),
    "block_idle": (SS + "block idle.fbx", dict(loop=True, face=MEHMED_STANCE, posture=MEHMED_POSTURE)),
    "jump": (SS + "jump (2).fbx", dict(kind="leap", arms_out=8)),
    "roll": (ROLL, dict(kind="leap", yaw_keep=0.0, face=0.0)),
    "hit": (SS + "impact.fbx", dict(kind="flinch", face=MEHMED_STANCE, posture=MEHMED_POSTURE)),
}

# ------------------------------------------------------------------ Miyamoto

# The kenjutsu guard: near square to the foe (his hips 15 degrees to his right), the katana held
# before him in both fists (`anim::hold` closes the off hand on the hilt in the game; the clips keep
# the source's arms). His shield is slung on his back and comes before him only to guard, in the
# game (`hold::sling`): the clips leave it be.
MIYAMOTO_STANCE = -15.0

MIYAMOTO = {
    "idle": (GS + "idle.fbx", dict(face=MIYAMOTO_STANCE, loop=True, arms_out=4)),
    "walk": (GS + "walk.fbx", dict(face=MIYAMOTO_STANCE, loop=True, stride=1.3, arms_out=4)),
    "run": (GS + "run.fbx", dict(loop=True, stride=3.0, arms_out=4)),
    # The forehand: a diagonal cut (kesa-giri) from high at his right down through the foe.
    "slash_fore": (GS + "slash.fbx", dict(kind="strike", face=MIYAMOTO_STANCE, arms_out=4)),
    # The backhand: a level cut from his left out to his right at the height of his shoulders.
    "slash_back": (GS + "slash (3).fbx", dict(kind="strike", face=MIYAMOTO_STANCE, arms_out=4)),
    # The finisher: the leaping cut, its whirls taken out: he springs and brings the blade down.
    "slash_heavy": (GS + "jump attack.fbx", dict(kind="strike", yaw_keep=0.0, still=True, face=MIYAMOTO_STANCE, arms_out=4)),
    "block": (GS + "blocking.fbx", dict(kind="raise", slung=True, face=MIYAMOTO_STANCE)),
    "block_idle": (GS + "blocking (2).fbx", dict(loop=True, face=MIYAMOTO_STANCE)),
    "jump": (GS + "jump (2).fbx", dict(kind="leap", arms_out=4)),
    "roll": (ROLL, dict(kind="leap", yaw_keep=0.0, face=0.0)),
    "hit": (GS + "impact (2).fbx", dict(kind="flinch", face=MIYAMOTO_STANCE)),
}

HEROES = {"ragnar": RAGNAR, "mehmed": MEHMED, "miyamoto": MIYAMOTO}


# ------------------------------------------------------------------ markers


def tip_track(arm, act, weapon="blade", rest=None):
    """The blade's tip (game axes) every frame of `act`, the feet's and hips' heights, the shield's
    and the off hand's joints, and the chest's turn. `weapon` is the bone of the weapon that strikes
    and `rest` the way it points at rest (game axes; the bone's own +Y where not given)."""
    scene = bpy.context.scene
    arm.animation_data.action = act
    f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
    out = {k: [] for k in ("tips", "feet", "hips", "shield", "hand", "chest")}
    pb = arm.pose.bones[weapon]
    tip_rest = None if rest is None else pb.bone.head_local + vh.g2b(Vector(rest).normalized() * 0.35)
    for f in range(f0, f1 + 1):
        scene.frame_set(f)
        m = arm.matrix_world @ pb.matrix
        if tip_rest is None:
            out["tips"].append(vh.b2g(m @ Vector((0, 0.3, 0))))
        else:
            out["tips"].append(vh.b2g(m @ pb.bone.matrix_local.inverted() @ tip_rest))
        feet = [b for b in ("foot.L", "foot.R") if b in arm.pose.bones]
        out["feet"].append(min((vh.b2g(arm.matrix_world @ arm.pose.bones[b].head).y for b in feet), default=0.0))
        out["hips"].append(vh.b2g(arm.matrix_world @ arm.pose.bones["hips"].head).y)
        shield = arm.pose.bones.get("shield") or arm.pose.bones["hand.L"]
        out["shield"].append(vh.b2g(arm.matrix_world @ shield.head))
        out["hand"].append(vh.b2g(arm.matrix_world @ arm.pose.bones["hand.L"].head))
        out["chest"].append((arm.matrix_world @ arm.pose.bones["chest"].matrix).to_quaternion())
    return out


def foot_stride(arm, act):
    """The ground a gait's cycle covers, from its feet: each frame the lower foot is planted, and
    how far it slides back under the hips is how far the body goes forward over it (metres)."""
    scene = bpy.context.scene
    arm.animation_data.action = act
    f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
    feet = []
    for f in range(f0, f1 + 1):
        scene.frame_set(f)
        feet.append([vh.b2g(arm.matrix_world @ arm.pose.bones[b].head) for b in ("foot.L", "foot.R")])
    ground = min(min(p.y for p in pair) for pair in feet)
    total = 0.0
    for a, b in zip(feet, feet[1:]):
        k = 0 if a[0].y <= a[1].y else 1
        # (Only while that foot is on the floor: a foot in the air swings forward.)
        if a[k].y < ground + 0.03:
            total += max(0.0, a[k].z - b[k].z)
    return total


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


def raise_marks(track):
    sp = [(track[i + 1] - track[i]).length for i in range(len(track) - 1)]
    k = max(range(len(sp)), key=lambda i: sp[i])
    r = next((i for i in range(k, len(sp)) if sp[i] < sp[k] * 0.2), len(sp) - 1)
    return {"raised": r + 1}


def flinch_marks(chest):
    sp = [chest[i].rotation_difference(chest[i + 1]).angle for i in range(len(chest) - 1)]
    return {"impact": max(range(len(sp)), key=lambda i: sp[i]) + 1}


def trim(act, start, end=None):
    """Cuts the first `start` frames off `act` (keys and markers move down with it), and with `end`
    everything after that frame (counted before the cut)."""
    for fc in vh.fcurves(act):
        # (Removed from the last back, so the points not yet looked at keep their places.)
        for i in reversed(range(len(fc.keyframe_points))):
            x = fc.keyframe_points[i].co.x
            if x < start - 1e-3 or (end is not None and x > end + 1e-3):
                fc.keyframe_points.remove(fc.keyframe_points[i])
        for kp in fc.keyframe_points:
            kp.co.x -= start
            kp.handle_left.x -= start
            kp.handle_right.x -= start
    f0, f1 = act.frame_range
    act.frame_range = (0, (end if end is not None else f1) - start)


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


def make(slot, path, opts, tgt, prefix="mixamo_hero"):
    old = bpy.data.actions.get(slot)
    if old:
        bpy.data.actions.remove(old)
    src, fps = rt.load_source(path)
    r = rt.Retarget(src, tgt, arms_out=opts.get("arms_out", 0.0), turn=opts.get("turn", 0.0), yaw_keep=opts.get("yaw_keep", 1.0),
                    hold={"blade": opts.get("hold", HOLD), "offhand": opts.get("hold", HOLD)}, sword_turn=opts.get("sword_turn", (0.0, 0.0)),
                    face=opts.get("face"), posture=opts.get("posture"), mirror=opts.get("mirror", False), weapon_rest=opts.get("weapon_rest"))
    frames = None
    if "span" in opts:
        f0 = int(src.animation_data.action.frame_range[0])
        frames = (f0 + round(opts["span"][0] * fps), f0 + round(opts["span"][1] * fps))
    act = rt.bake(r, slot, loop=opts.get("loop", False), stride=None if opts.get("stride") == "feet" else opts.get("stride"), fps_src=fps, frames=frames, still=opts.get("still", False))
    # (The source and its own action go: only his is kept.)
    for o in [o for o in bpy.data.objects if o.parent == src] + [src]:
        bpy.data.objects.remove(o)
    for leftover in {x.name for x in bpy.data.actions if x.name.startswith("Armature|")}:
        bpy.data.actions.remove(bpy.data.actions[leftover])
    if opts.get("reverse"):
        reverse(act)
    if opts.get("stride") == "feet":
        act["vh_stride"] = foot_stride(tgt, act)
        print(f"{prefix}: {slot}: its feet carry it {act['vh_stride']:.2f} m a cycle")
    kind = opts.get("kind")
    marks = {}
    if kind:
        t = tip_track(tgt, act, opts.get("weapon", "blade"), opts.get("tip_rest", opts.get("weapon_rest")))
        if kind == "strike":
            marks = strike_marks(t["tips"])
            lead = marks.pop("lead")
            trim(act, lead, opts.get("cut"))
            marks = {k: v - lead for k, v in marks.items()}
            marks["recover_end"] = int(act.frame_range[1])
        elif kind == "leap":
            marks = leap_marks(t["feet"], t["hips"])
            marks["recover_end"] = int(act.frame_range[1])
        elif kind == "raise":
            marks = raise_marks(t["hand" if opts.get("slung") else "shield"])
        elif kind == "flinch":
            if opts.get("cut"):
                # (Only the reaction: the long settle after it is cut off.)
                trim(act, 0, opts["cut"])
                t = tip_track(tgt, act, opts.get("weapon", "blade"), opts.get("tip_rest", opts.get("weapon_rest")))
            marks = flinch_marks(t["chest"])
            marks["recover_end"] = int(act.frame_range[1])
    for k, v in marks.items():
        act.pose_markers.new(k).frame = int(v)
    act["vh_source"] = os.path.basename(path)
    for k, v in opts.get("extra_marks", {}).items():
        act.pose_markers.new(k).frame = int(v)
    print(f"{prefix}: {slot:12} from {os.path.basename(path):40} {int(act.frame_range[1]):4} frames, markers {marks}")
    return act


def parry(tgt):
    """The parry: Mixamo has none. The guard snapped up (the block's raised pose, reached in 3
    frames, against the block's 10), then the recoil from the blow it turned (the hit flinch on his
    upper body, half as deep), and back."""
    old = bpy.data.actions.get("parry")
    if old:
        bpy.data.actions.remove(old)
    block, hit = bpy.data.actions["block"], bpy.data.actions["hit"]
    raised = int(block.pose_markers["raised"].frame)
    impact = int(hit.pose_markers["impact"].frame)
    scene = bpy.context.scene

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
    print("mixamo_hero: parry        from the block and the flinch, 22 frames")


# ------------------------------------------------------------------ the blade from the chase camera

# The chase camera at rest, from him (game axes, metres): behind him, above, over his right shoulder
# (`camera::chase`: the pivot 1.7 up and 0.3 to his right, 3.4 back along a pitch of -0.44).
CHASE_EYE = (-0.3, 1.7 + 3.4 * math.sin(0.44), -3.4 * math.cos(0.44))


def measure(arm, names):
    """For each strike, the share of its active frames on which some of the blade is in plain sight
    of the chase camera: a frame counts when at least a quarter of the points along the blade's voxels
    can be reached by a ray from the eye without striking his body first."""
    scene = bpy.context.scene
    blade = bpy.data.objects.get("blade_voxels")
    if blade is None:
        sys.exit("measure: the .blend has no voxels (save it with import_rig.py)")
    eye = vh.g2b(CHASE_EYE)
    results = {}
    for name in names:
        act = bpy.data.actions.get(name)
        if act is None:
            continue
        arm.animation_data.action = act
        a, b = int(act.pose_markers["windup_end"].frame), int(act.pose_markers["active_end"].frame)
        seen = 0
        for f in range(a, b + 1):
            scene.frame_set(f)
            dg = bpy.context.evaluated_depsgraph_get()
            ev = blade.evaluated_get(dg)
            pts = [ev.matrix_world @ v.co for v in ev.data.vertices[:: max(1, len(ev.data.vertices) // 60)]]
            clear = 0
            for p in pts:
                d = p - eye
                hit, loc, _, _, obj, _ = scene.ray_cast(dg, eye, d.normalized(), distance=d.length - 0.004)
                if not hit or obj.name.startswith("blade"):
                    clear += 1
            seen += clear >= len(pts) / 4
        results[name] = (seen, b - a + 1)
        print(f"measure: {name:12} the blade seen on {seen} of {b - a + 1} active frames ({100 * seen // max(1, b - a + 1)}%)")
    return results


def main():
    a = vh.args()
    if len(a) < 2:
        sys.exit(__doc__)
    hero = a[0]
    if hero not in HEROES:
        sys.exit(f"mixamo_hero: no hero {hero} (ragnar, mehmed or miyamoto)")
    tgt = vh.armature()
    if tgt.animation_data is None:
        tgt.animation_data_create()
    if a[1] == "--measure":
        measure(tgt, a[2:] or ["slash_fore", "slash_back", "slash_heavy"])
        return
    root, out, rest = a[1], a[2], a[3:]
    if rest and rest[0] == "--try":
        for item in rest[1:]:
            path, _, extra = item.partition(":")
            opts = eval(f"dict({extra})") if extra else {}
            label = os.path.splitext(os.path.basename(path))[0].replace("sword and shield ", "ss ").replace("great sword ", "gs ")
            make(label, os.path.join(root, path), opts, tgt)
    else:
        picks = HEROES[hero]
        only = set(rest)
        for slot, (path, opts) in picks.items():
            if only and slot not in only:
                continue
            make(slot, os.path.join(root, path), opts, tgt)
        if (not only or "parry" in only) and "block" in bpy.data.actions and "hit" in bpy.data.actions:
            parry(tgt)
    bpy.context.scene.render.fps = vh.FPS
    tgt.animation_data.action = bpy.data.actions.get("idle")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out))
    print("mixamo_hero: saved", out)


if __name__ == "__main__":
    main()
